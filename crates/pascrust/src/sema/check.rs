//! The main sema pass: name resolution and type checking.

use std::collections::{HashMap, HashSet};

use std::rc::Rc;

use super::names::{self, screaming_snake_case, snake_case};
use super::types::{ArrayType, EnumType, Field, ObjectType, RecordType};
use super::*;
use crate::ast::*;

/// What a name refers to.
enum Resolved {
    Sym(SymId),
    /// A field of `Self`, inside a method.
    SelfField {
        self_sym: SymId,
        path: Vec<String>,
        ty: Type,
    },
    /// A method of `Self`, inside a method.
    SelfMethod {
        self_sym: SymId,
        pid: ProcId,
        path: Vec<String>,
    },
}

/// One `for` loop, remembered so the lifting pass can decide whether its
/// variable can be scoped to the loop.
#[derive(Debug, Clone)]
pub(super) struct ForLoop {
    pub stmt_start: usize,
    pub var: SymId,
    pub proc: ProcId,
}

pub(super) struct Checker {
    a: Analysis,
    /// The scope stack. Each scope maps a lowercase name to a symbol. The
    /// first scope holds the builtins, the second the program's globals.
    scopes: Vec<HashMap<String, SymId>>,
    /// The procedures we're currently inside, innermost last.
    proc_stack: Vec<ProcId>,
    /// Loop variables of the `for` statements we're inside.
    active_for_vars: Vec<SymId>,
    /// Variables used somewhere other than inside a `for` they control.
    pub(super) used_outside_for: HashSet<SymId>,
    pub(super) for_loops: Vec<ForLoop>,
    /// Rust names already taken, per procedure. A procedure starts with a copy
    /// of its parent's set, so a lifted procedure's captured variables can
    /// never clash with its own locals.
    used_names: Vec<HashSet<String>>,
    used_fn_names: HashSet<String>,
    used_const_names: HashSet<String>,
    used_type_names: HashSet<String>,
    /// For each method body we're inside: the index of its scope in
    /// `scopes` and the method's procedure.
    method_scopes: Vec<(usize, ProcId)>,
    /// The Rust module name of the unit being checked, if any.
    current_unit: Option<String>,
    /// Each user unit's interface: what a `uses` clause makes visible.
    unit_exports: HashMap<String, HashMap<String, SymId>>,
    /// True while resolving the right-hand side of a `type` declaration.
    declaring_type: bool,
    /// Bounds of type names declared as subranges (`Index = 1..10`).
    subrange_types: HashMap<SymId, (Type, i64, i64)>,
    errors: Vec<Diagnostic>,
    call_sites: Vec<CallSite>,
}

impl Checker {
    pub fn new() -> Checker {
        let mut checker = Checker {
            a: Analysis::default(),
            scopes: vec![HashMap::new()],
            proc_stack: vec![MAIN],
            active_for_vars: Vec::new(),
            used_outside_for: HashSet::new(),
            for_loops: Vec::new(),
            used_names: vec![HashSet::new()],
            used_fn_names: HashSet::new(),
            used_const_names: HashSet::new(),
            used_type_names: HashSet::new(),
            declaring_type: false,
            unit_exports: HashMap::new(),
            current_unit: None,
            method_scopes: Vec::new(),
            subrange_types: HashMap::new(),
            errors: Vec::new(),
            call_sites: Vec::new(),
        };
        checker.a.procs.push(ProcInfo {
            name: "main program".to_string(),
            rust_name: "pascal_main".to_string(),
            kind: ProcKind::Procedure,
            parent: None,
            params: Vec::new(),
            ret: None,
            result: None,
            locals: Vec::new(),
            uses: Default::default(),
            writes: Default::default(),
            calls: Default::default(),
            captures: Vec::new(),
            has_body: true,
            self_param: None,
            span: Span::default(),
        });
        checker.declare_builtins();
        checker
    }

    pub fn finish(self) -> (Analysis, Vec<Diagnostic>, Vec<CallSite>) {
        let mut a = self.a;
        // Hand the `for` loop facts to the lifting pass through the analysis.
        for f in &self.for_loops {
            let only_in_loops = !self.used_outside_for.contains(&f.var);
            a.for_only.insert(f.var, only_in_loops);
        }
        a.for_loop_list = self.for_loops.iter().map(|f| (f.stmt_start, f.var, f.proc)).collect();
        (a, self.errors, self.call_sites)
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.errors.push(Diagnostic::new(span, message));
    }

    fn cur(&self) -> ProcId {
        *self.proc_stack.last().unwrap()
    }

    // ----- Symbols and scopes ----------------------------------------------

    fn add_symbol(&mut self, name: &str, rust_name: String, kind: SymKind) -> SymId {
        self.a.symbols.push(Symbol { name: name.to_string(), rust_name, kind });
        self.a.symbols.len() - 1
    }

    /// Put `sym` in the innermost scope under `ident`, reporting duplicates.
    fn bind(&mut self, ident: &Ident, sym: SymId) {
        let scope = self.scopes.last_mut().unwrap();
        if scope.insert(ident.key(), sym).is_some() && self.scopes.len() > 1 {
            self.error(ident.span, format!("`{}` is declared twice in the same scope", ident.name));
        }
        self.a.decls.insert(ident.span.start, sym);
    }

    fn lookup(&self, key: &str) -> Option<SymId> {
        self.scopes.iter().rev().find_map(|scope| scope.get(key).copied())
    }

    /// Look up a name the way a statement sees it: like [`Checker::lookup`],
    /// except that inside a method, the object's fields and methods are in
    /// scope too (Turbo Pascal's implicit `with Self do`). They come after the
    /// method's own parameters and locals but before anything outside it.
    fn resolve(&self, key: &str) -> Option<Resolved> {
        let method = self.method_scopes.last().copied();
        for (depth, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(&sym) = scope.get(key) {
                return Some(Resolved::Sym(sym));
            }
            if let Some((method_depth, pid)) = method
                && depth == method_depth
            {
                let (object, self_sym) = self.a.procs[pid].self_param.clone().expect("a method has a Self");
                if let Some((path, ty)) = object.field_path(key) {
                    return Some(Resolved::SelfField { self_sym, path, ty });
                }
                if let Some((pid, path)) = self.find_method(&object, key) {
                    return Some(Resolved::SelfMethod { self_sym, pid, path });
                }
            }
        }
        None
    }

    /// A method of `object` or one of its ancestors, with the path to the
    /// ancestor that defines it.
    fn find_method(&self, object: &Rc<ObjectType>, key: &str) -> Option<(ProcId, Vec<String>)> {
        object
            .lineage()
            .into_iter()
            .find_map(|(o, path)| self.a.methods.get(&(o.rust_name.clone(), key.to_string())).map(|&pid| (pid, path)))
    }

    /// Pick a Rust name not yet used in `proc` (or any procedure enclosing
    /// it), by appending `_2`, `_3`, ... if needed.
    fn unique_var_name(&mut self, proc: ProcId, base: String) -> String {
        let used = &mut self.used_names[proc];
        let mut name = base.clone();
        let mut n = 2;
        while used.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        used.insert(name.clone());
        name
    }

    fn unique_in(set: &mut HashSet<String>, base: String) -> String {
        let mut name = base.clone();
        let mut n = 2;
        while set.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        set.insert(name.clone());
        name
    }

    fn declare_var(&mut self, ident: &Ident, ty: Type, owner: ProcId, role: VarRole) -> SymId {
        let rust_name = self.unique_var_name(owner, snake_case(&ident.name));
        let sym = self.add_symbol(&ident.name, rust_name, SymKind::Var(Var { ty, owner, role }));
        self.bind(ident, sym);
        sym
    }

    fn declare_builtins(&mut self) {
        let types = [
            ("integer", Type::Int(IntKind::Integer)),
            ("longint", Type::Int(IntKind::LongInt)),
            ("shortint", Type::Int(IntKind::ShortInt)),
            ("byte", Type::Int(IntKind::Byte)),
            ("word", Type::Int(IntKind::Word)),
            ("real", Type::Real),
            ("double", Type::Real),
            ("extended", Type::Real),
            ("single", Type::Real),
            ("boolean", Type::Bool),
            ("char", Type::Char),
        ];
        for (name, ty) in types {
            let rust = ty.rust();
            let sym = self.add_symbol(name, rust, SymKind::TypeName(ty));
            self.scopes[0].insert(name.to_string(), sym);
        }
        let consts = [
            ("true", ConstVal::Bool(true), Type::Bool, "true"),
            ("false", ConstVal::Bool(false), Type::Bool, "false"),
            ("maxint", ConstVal::Int(32767), Type::Int(IntKind::Integer), "i16::MAX"),
            ("maxlongint", ConstVal::Int(i32::MAX as i64), Type::Int(IntKind::LongInt), "i32::MAX"),
        ];
        for (name, value, ty, rust) in consts {
            let sym = self.add_symbol(name, rust.to_string(), SymKind::Const { value, ty });
            self.scopes[0].insert(name.to_string(), sym);
        }
        for &(name, builtin) in Builtin::ALL {
            let sym = self.add_symbol(name, name.to_string(), SymKind::Builtin(builtin));
            self.scopes[0].insert(name.to_string(), sym);
        }
        self.declare_unit_routines(super::units::SYSTEM, &[]);
    }

    /// Make a standard unit's routines and constants visible.
    fn declare_unit_routines(&mut self, routines: &'static [super::units::UnitRoutine], consts: &[(&str, &str, i64)]) {
        for routine in routines {
            let sym = self.add_symbol(routine.name, routine.rust.to_string(), SymKind::UnitRoutine(routine));
            self.scopes[0].insert(routine.name.to_string(), sym);
        }
        for &(name, rust, value) in consts {
            let ty = Type::Int(IntKind::Byte);
            let sym = self.add_symbol(name, rust.to_string(), SymKind::Const { value: ConstVal::Int(value), ty });
            self.scopes[0].insert(name.to_string(), sym);
        }
    }

    // ----- Programs and declarations ---------------------------------------

    pub fn program(&mut self, program: &Program, units: &[Unit]) {
        for unit in units {
            self.unit(unit);
        }
        self.use_units(&program.uses);
        self.scopes.push(HashMap::new());
        self.block(&program.block);
    }

    /// Make the units in a `uses` clause visible: standard units' routines,
    /// or a user unit's interface (as a new scope, so a later unit's names
    /// hide an earlier one's, which is Turbo Pascal's rule).
    fn use_units(&mut self, uses: &[Ident]) {
        for unit in uses {
            if let Some((routines, consts)) = super::units::unit(&unit.key()) {
                self.declare_unit_routines(routines, consts);
            } else if let Some(exports) = self.unit_exports.get(&unit.key()) {
                self.scopes.push(exports.clone());
            } else {
                self.error(unit.span, format!("unit `{}` is not supported yet", unit.name));
            }
        }
    }

    fn unit(&mut self, unit: &Unit) {
        self.current_unit = Some(snake_case(&unit.name.name));
        let depth = self.scopes.len();
        self.use_units(&unit.interface_uses);
        self.scopes.push(HashMap::new());
        self.decls(&unit.interface);
        // Everything declared so far is the unit's public interface.
        let exports = self.scopes.last().unwrap().clone();
        let own_scope = self.scopes.pop().unwrap();
        self.use_units(&unit.implementation_uses);
        self.scopes.push(own_scope);
        self.decls(&unit.implementation);
        self.check_forwards();

        // The initialization part runs before the main program. It's
        // treated as a procedure of its own, so it can be lifted like one.
        let mut init = None;
        if !unit.init.is_empty() {
            let pid = self.a.procs.len();
            let rust_name = Self::unique_in(&mut self.used_fn_names, format!("{}_init", snake_case(&unit.name.name)));
            self.a.procs.push(ProcInfo {
                name: format!("{} initialization", unit.name.name),
                rust_name,
                kind: ProcKind::Procedure,
                parent: Some(MAIN),
                params: Vec::new(),
                ret: None,
                result: None,
                locals: Vec::new(),
                uses: Default::default(),
                writes: Default::default(),
                calls: Default::default(),
                captures: Vec::new(),
                has_body: true,
                self_param: None,
                span: unit.name.span,
            });
            self.used_names.push(HashSet::new());
            self.a.procs[MAIN].calls.insert(pid);
            self.proc_stack.push(pid);
            self.stmts(&unit.init);
            self.proc_stack.pop();
            init = Some(pid);
        }
        self.scopes.truncate(depth);
        self.current_unit = None;
        self.unit_exports.insert(unit.name.key(), exports);
        self.a.units.push(UnitInfo {
            name: unit.name.name.clone(),
            module: snake_case(&unit.name.name),
            init,
            span: unit.span,
        });
    }

    fn block(&mut self, block: &Block) {
        self.decls(&block.decls);
        self.check_forwards();
        self.stmts(&block.body);
    }

    fn decls(&mut self, decls: &[Decl]) {
        for decl in decls {
            match decl {
                Decl::Const(c) => self.const_decl(c),
                Decl::Type(t) if matches!(t.ty, TypeExpr::Object { .. }) => self.object_type(&t.name, &t.ty),
                Decl::Type(t) => {
                    let bounds = match &t.ty {
                        TypeExpr::Subrange { .. } => self.subrange(&t.ty),
                        _ => None,
                    };
                    let ty = match &bounds {
                        Some((host, _, _)) => host.clone(),
                        None => {
                            self.declaring_type = true;
                            let ty = self.resolve_named_type(&t.ty, Some(&t.name.name));
                            self.declaring_type = false;
                            ty
                        }
                    };
                    let sym = self.add_symbol(&t.name.name, ty.rust(), SymKind::TypeName(ty));
                    self.bind(&t.name, sym);
                    if let Some(bounds) = bounds {
                        self.subrange_types.insert(sym, bounds);
                    }
                }
                Decl::Var(v) => {
                    let ty = self.resolve_named_type(&v.ty, Some(&v.names[0].name));
                    let owner = self.cur();
                    for name in &v.names {
                        let sym = self.declare_var(name, ty.clone(), owner, VarRole::Local);
                        self.a.procs[owner].locals.push(sym);
                    }
                }
                Decl::Proc(p) => self.proc_decl(p),
            }
        }
    }

    /// Every procedure declared `forward` (or in a unit interface) in the
    /// current scope must have been given a body.
    fn check_forwards(&mut self) {
        let mut missing: Vec<(Span, String)> = Vec::new();
        for (key, &sym) in self.scopes.last().unwrap() {
            if let SymKind::Proc(p) = self.a.symbols[sym].kind
                && !self.a.procs[p].has_body
            {
                missing.push((self.a.procs[p].span, key.clone()));
            }
        }
        missing.sort_by_key(|(span, _)| span.start);
        for (span, key) in missing {
            self.error(span, format!("`{key}` is declared but its body is never defined"));
        }
    }

    fn const_decl(&mut self, c: &ConstDecl) {
        if let Some(ty_expr) = &c.ty {
            // A typed constant is really an initialized variable. In Turbo
            // Pascal it lives in the data segment even when declared inside a
            // procedure, so it keeps its value between calls. We get the
            // same effect by making it a global, i.e. a local of `main`.
            let ty = self.resolve_type(ty_expr);
            self.const_value(&c.value, &ty);
            let sym = self.declare_var(&c.name, ty, MAIN, VarRole::Local);
            self.a.procs[MAIN].locals.push(sym);
            self.a.typed_consts.insert(sym, c.clone());
            self.use_var(sym, false);
            return;
        }
        let ConstValue::Expr(e) = &c.value else {
            self.error(c.name.span, "only typed constants can have a list of values");
            return;
        };
        let Some(value) = self.eval_const(e) else {
            self.error(e.span, "expected a constant expression");
            return;
        };
        let ty = match &value {
            ConstVal::Int(v) => Type::Int(IntKind::for_literal(*v)),
            ConstVal::Real(_) => Type::Real,
            ConstVal::Bool(_) => Type::Bool,
            ConstVal::Str(s) if s.len() == 1 => Type::Char,
            ConstVal::Str(s) => Type::StrLit(s.len()),
        };
        let rust_name = Self::unique_in(&mut self.used_const_names, screaming_snake_case(&c.name.name));
        let sym = self.add_symbol(&c.name.name, rust_name, SymKind::Const { value, ty });
        self.bind(&c.name, sym);
    }

    /// Check the initializer of a typed constant against its type.
    fn const_value(&mut self, value: &ConstValue, ty: &Type) {
        match (value, ty) {
            (ConstValue::Expr(e), _) => {
                let value_ty = self.expr(e);
                if !ty.accepts(&value_ty) {
                    self.error(e.span, format!("expected a {ty} value, found {value_ty}"));
                }
            }
            (ConstValue::List(items, span), Type::Array(a)) => {
                if items.len() as i64 != a.count() {
                    self.error(*span, format!("expected {} values for this array, found {}", a.count(), items.len()));
                }
                for item in items {
                    self.const_value(item, &a.elem);
                }
            }
            (ConstValue::Record(fields, _), Type::Record(r)) => {
                for (name, value) in fields {
                    match r.field(&name.name) {
                        Some(field) => self.const_value(value, &field.ty.clone()),
                        None => self.error(name.span, format!("`{}` has no field `{}`", r.rust_name, name.name)),
                    }
                }
            }
            (ConstValue::List(_, span) | ConstValue::Record(_, span), _) => {
                self.error(*span, format!("this initializer doesn't fit type {ty}"));
            }
        }
    }

    fn proc_decl(&mut self, d: &ProcDecl) {
        if d.owner.is_some() {
            return self.method_body(d);
        }
        let parent = self.cur();
        // A procedure declared `forward` earlier in this scope gets its body
        // now; reuse its ProcInfo (and its parameters).
        let forward = self.scopes.last().unwrap().get(&d.name.key()).and_then(|&sym| match self.a.symbols[sym].kind {
            SymKind::Proc(p) if !self.a.procs[p].has_body => Some(p),
            _ => None,
        });
        let pid = match forward {
            Some(p) => p,
            None => self.new_proc(d, parent),
        };
        self.a.proc_decls.insert(d.name.span.start, pid);

        let ProcBody::Block(body) = &d.body else {
            if d.body == ProcBody::External {
                self.error(d.name.span, "`external` procedures are not supported yet");
            }
            return;
        };
        self.a.procs[pid].has_body = true;
        self.scopes.push(HashMap::new());
        for &param in &self.a.procs[pid].params.clone() {
            let key = self.a.symbols[param].name.to_ascii_lowercase();
            self.scopes.last_mut().unwrap().insert(key, param);
        }
        self.proc_stack.push(pid);
        self.block(body);
        self.proc_stack.pop();
        self.scopes.pop();
    }

    /// `Name = object(Parent) fields; method headings; end`.
    fn object_type(&mut self, name: &Ident, t: &TypeExpr) {
        let TypeExpr::Object { parent, fields, methods, span } = t else { unreachable!() };
        let parent = parent.as_ref().and_then(|p| match self.lookup(&p.key()).map(|s| &self.a.symbols[s].kind) {
            Some(SymKind::TypeName(Type::Object(o))) => Some(o.clone()),
            _ => {
                self.error(p.span, format!("`{}` is not an object type", p.name));
                None
            }
        });
        let rust_name = self.type_name(&name.name);
        let mut resolved = Vec::new();
        for group in fields {
            let ty = self.resolve_type(&group.ty);
            for n in &group.names {
                resolved.push(Field { name: n.name.clone(), rust_name: snake_case(&n.name), ty: ty.clone() });
            }
        }
        let object = Rc::new(ObjectType { rust_name: rust_name.clone(), parent, fields: resolved });
        let ty = Type::Object(object.clone());
        // Bind the type's name before looking at the methods, whose
        // parameters may mention the type itself.
        let sym = self.add_symbol(&name.name, rust_name.clone(), SymKind::TypeName(ty.clone()));
        self.bind(name, sym);
        self.a.type_defs.push((ty, *span));
        for method in methods {
            let pid = self.new_proc_inner(method, MAIN, false);
            let self_sym = self.add_symbol(
                "Self",
                "self".to_string(),
                SymKind::Var(Var {
                    ty: Type::Object(object.clone()),
                    owner: pid,
                    role: VarRole::Param(ParamMode::Var),
                }),
            );
            self.used_names[pid].insert("self".to_string());
            let info = &mut self.a.procs[pid];
            info.self_param = Some((object.clone(), self_sym));
            info.name = format!("{}.{}", name.name, method.name.name);
            // Methods live in their type's namespace, so they don't need the
            // globally unique names that free functions do.
            self.used_fn_names.remove(&info.rust_name);
            info.rust_name = snake_case(&method.name.name);
            if self.a.methods.insert((rust_name.clone(), method.name.key()), pid).is_some() {
                self.error(method.name.span, format!("`{}` is declared twice in `{}`", method.name.name, name.name));
            }
        }
    }

    /// `procedure Queue.AddQ(...); begin ... end;`: a method's body.
    fn method_body(&mut self, d: &ProcDecl) {
        let owner = d.owner.as_ref().unwrap();
        let object = match self.lookup(&owner.key()).map(|s| &self.a.symbols[s].kind) {
            Some(SymKind::TypeName(Type::Object(o))) => o.clone(),
            _ => {
                self.error(owner.span, format!("`{}` is not an object type", owner.name));
                return;
            }
        };
        let Some(&pid) = self.a.methods.get(&(object.rust_name.clone(), d.name.key())) else {
            self.error(d.name.span, format!("`{}` has no method `{}`", owner.name, d.name.name));
            return;
        };
        let ProcBody::Block(body) = &d.body else {
            self.error(d.name.span, "a method implementation needs a body");
            return;
        };
        if self.a.procs[pid].has_body {
            self.error(d.name.span, format!("method `{}.{}` is defined twice", owner.name, d.name.name));
        }
        self.a.proc_decls.insert(d.name.span.start, pid);
        self.a.procs[pid].has_body = true;
        self.scopes.push(HashMap::new());
        for &param in &self.a.procs[pid].params.clone() {
            let key = self.a.symbols[param].name.to_ascii_lowercase();
            self.scopes.last_mut().unwrap().insert(key, param);
        }
        let self_sym = self.a.procs[pid].self_param.as_ref().unwrap().1;
        self.scopes.last_mut().unwrap().insert("self".to_string(), self_sym);
        self.method_scopes.push((self.scopes.len() - 1, pid));
        self.proc_stack.push(pid);
        self.block(body);
        self.proc_stack.pop();
        self.method_scopes.pop();
        self.scopes.pop();
    }

    fn new_proc(&mut self, d: &ProcDecl, parent: ProcId) -> ProcId {
        self.new_proc_inner(d, parent, true)
    }

    /// Create a procedure's `ProcInfo` and parameters. Methods aren't bound
    /// in the enclosing scope (`bind` is false): they're reached through
    /// their object.
    fn new_proc_inner(&mut self, d: &ProcDecl, parent: ProcId, bind: bool) -> ProcId {
        let pid = self.a.procs.len();
        // Nested procedures are named after their parents: `Visit` inside
        // `Report` becomes `report_visit`, so lifted names never collide.
        let base = if parent == MAIN {
            snake_case(&d.name.name)
        } else if let Some((object, _)) = &self.a.procs[parent].self_param {
            // Nested inside a method: `Queue.AddQ`'s `Helper` becomes
            // `queue_add_q_helper`.
            format!("{}_{}_{}", snake_case(&object.rust_name), self.a.procs[parent].rust_name, snake_case(&d.name.name))
        } else {
            format!("{}_{}", self.a.procs[parent].rust_name, snake_case(&d.name.name))
        };
        // Two units may both declare a `Name` function. The second one gets
        // its unit's name as a prefix (`labels_name`).
        let base = match &self.current_unit {
            Some(unit) if self.used_fn_names.contains(&base) => format!("{unit}_{base}"),
            _ => base,
        };
        let rust_name = Self::unique_in(&mut self.used_fn_names, base);
        let ret = d.ret.as_ref().map(|t| self.resolve_type(t));
        if d.kind == ProcKind::Function && ret.is_none() {
            self.error(d.name.span, "a function needs a result type");
        }
        self.a.procs.push(ProcInfo {
            name: d.name.name.clone(),
            rust_name: rust_name.clone(),
            kind: d.kind,
            parent: Some(parent),
            params: Vec::new(),
            ret: ret.clone(),
            result: None,
            locals: Vec::new(),
            uses: Default::default(),
            writes: Default::default(),
            calls: Default::default(),
            captures: Vec::new(),
            has_body: false,
            self_param: None,
            span: d.name.span,
        });
        // Each procedure has its own namespace for variables. If a lifted
        // procedure's captured variable clashes with one of its own, the
        // lifting pass renames the captured one.
        self.used_names.push(HashSet::new());
        self.used_names[pid].insert("result".to_string());

        let sym = self.add_symbol(&d.name.name, rust_name, SymKind::Proc(pid));
        if bind {
            self.bind(&d.name, sym);
        }

        // Parameters are declared in a throwaway scope here; `proc_decl`
        // re-binds them in the body's scope.
        self.scopes.push(HashMap::new());
        for group in &d.params {
            let ty = match &group.ty {
                Some(t) => self.resolve_type(t),
                None => {
                    self.error(group.names[0].span, "untyped parameters are not supported");
                    Type::Error
                }
            };
            for name in &group.names {
                let param = self.declare_var(name, ty.clone(), pid, VarRole::Param(group.mode));
                self.a.procs[pid].params.push(param);
            }
        }
        self.scopes.pop();

        if let Some(ret) = ret {
            let result = self.add_symbol(
                &d.name.name,
                "result".to_string(),
                SymKind::Var(Var { ty: ret, owner: pid, role: VarRole::Result }),
            );
            self.a.procs[pid].result = Some(result);
            self.a.procs[pid].locals.push(result);
        }
        pid
    }

    fn resolve_type(&mut self, t: &TypeExpr) -> Type {
        self.resolve_named_type(t, None)
    }

    /// Resolve a type expression. `name` is the Pascal name the type is
    /// being declared under (`type GridT = ...`), or the variable it's
    /// declared for, which names the Rust struct/enum/alias we generate.
    fn resolve_named_type(&mut self, t: &TypeExpr, name: Option<&str>) -> Type {
        match t {
            TypeExpr::Named(ident) => match self.lookup(&ident.key()).map(|s| &self.a.symbols[s].kind) {
                Some(SymKind::TypeName(ty)) => ty.clone(),
                Some(_) => {
                    self.error(ident.span, format!("`{}` is not a type", ident.name));
                    Type::Error
                }
                None => {
                    self.error(ident.span, format!("unknown type `{}`", ident.name));
                    Type::Error
                }
            },
            TypeExpr::String(None, _) => Type::Str { cap: 255 },
            TypeExpr::String(Some(len), span) => match self.eval_const(len) {
                Some(ConstVal::Int(n)) if (1..=255).contains(&n) => Type::Str { cap: n as u8 },
                _ => {
                    self.error(*span, "a string's maximum length must be a constant from 1 to 255");
                    Type::Error
                }
            },
            // A subrange variable is stored as its host type; the bounds only
            // matter when the subrange is used as an array index.
            TypeExpr::Subrange { .. } => match self.subrange(t) {
                Some((host, _, _)) => host,
                None => Type::Error,
            },
            TypeExpr::Enum(names, _) => {
                let rust_name = self.type_name(name.unwrap_or("Enum"));
                let variants = names.iter().map(|n| names::camel_case(&n.name)).collect();
                let ty = Type::Enum(Rc::new(EnumType { rust_name, variants }));
                // Each enum value is a constant in the enclosing scope, valued
                // by its position: `Red` is 0, `Green` 1, ...
                let Type::Enum(e) = &ty else { unreachable!() };
                for (i, n) in names.iter().enumerate() {
                    let rust = format!("{}::{}", e.rust_name, e.variants[i]);
                    let sym = self.add_symbol(
                        &n.name,
                        rust,
                        SymKind::Const { value: ConstVal::Int(i as i64), ty: ty.clone() },
                    );
                    self.bind(n, sym);
                }
                self.a.type_defs.push((ty.clone(), t.span()));
                ty
            }
            TypeExpr::Array { indexes, elem, .. } => {
                let elem_ty = self.resolve_type(elem);
                // `array[a, b] of T` is shorthand for `array[a] of array[b] of T`,
                // so build the types from the innermost index outward.
                let mut ty = elem_ty;
                for (i, index) in indexes.iter().enumerate().rev() {
                    let Some((index_ty, lo, hi)) = self.index_type(index) else { return Type::Error };
                    // Only a `type` declaration gets a Rust type alias; an
                    // array declared directly on a variable stays anonymous.
                    let alias = if i == 0 && self.declaring_type { name.map(|n| self.type_name(n)) } else { None };
                    ty = Type::Array(Rc::new(ArrayType { index: index_ty, lo, hi, elem: ty, alias }));
                }
                if name.is_some() && self.declaring_type {
                    self.a.type_defs.push((ty.clone(), t.span()));
                }
                ty
            }
            TypeExpr::Record(fields, _) => {
                let rust_name = self.type_name(name.unwrap_or("Record"));
                let mut resolved: Vec<Field> = Vec::new();
                for group in fields {
                    let ty = self.resolve_type(&group.ty);
                    for n in &group.names {
                        if resolved.iter().any(|f| f.name.eq_ignore_ascii_case(&n.name)) {
                            self.error(n.span, format!("duplicate field `{}`", n.name));
                        }
                        resolved.push(Field { name: n.name.clone(), rust_name: snake_case(&n.name), ty: ty.clone() });
                    }
                }
                let ty = Type::Record(Rc::new(RecordType { rust_name, fields: resolved }));
                self.a.type_defs.push((ty.clone(), t.span()));
                ty
            }
            other => {
                self.error(other.span(), "this kind of type is not supported yet");
                Type::Error
            }
        }
    }

    /// A unique Rust type name (`CamelCase`) for a Pascal name.
    fn type_name(&mut self, pascal: &str) -> String {
        Self::unique_in(&mut self.used_type_names, names::camel_case(pascal))
    }

    /// The host type and bounds of a subrange type expression `lo..hi`.
    fn subrange(&mut self, t: &TypeExpr) -> Option<(Type, i64, i64)> {
        let TypeExpr::Subrange { lo, hi, span } = t else { return None };
        let lo_ty = self.expr(lo);
        let (Some(ConstVal::Int(l)), Some(ConstVal::Int(h))) = (self.ordinal_const(lo), self.ordinal_const(hi)) else {
            self.error(*span, "subrange bounds must be ordinal constants");
            return None;
        };
        if l > h {
            self.error(*span, "a subrange's lower bound must not exceed its upper bound");
            return None;
        }
        let host = match lo_ty.concrete() {
            Type::Int(k) => Type::Int(if IntKind::Integer.contains(l) && IntKind::Integer.contains(h) {
                IntKind::Integer
            } else {
                k
            }),
            other => other,
        };
        Some((host, l, h))
    }

    /// A constant's ordinal value (characters count by their code).
    fn ordinal_const(&self, e: &Expr) -> Option<ConstVal> {
        match self.eval_const(e)? {
            ConstVal::Str(s) if s.len() == 1 => Some(ConstVal::Int(s[0] as i64)),
            ConstVal::Bool(b) => Some(ConstVal::Int(b as i64)),
            v @ ConstVal::Int(_) => Some(v),
            _ => None,
        }
    }

    /// The type and bounds of an array index: a subrange, or the name of an
    /// ordinal type with a small range (`Char`, `Boolean`, an enum, `Byte`).
    fn index_type(&mut self, t: &TypeExpr) -> Option<(Type, i64, i64)> {
        if let TypeExpr::Subrange { .. } = t {
            return self.subrange(t);
        }
        if let TypeExpr::Named(ident) = t
            && let Some(sym) = self.lookup(&ident.key())
            && let Some(bounds) = self.subrange_types.get(&sym)
        {
            return Some(bounds.clone());
        }
        let ty = self.resolve_type(t);
        match &ty {
            Type::Char => Some((ty, 0, 255)),
            Type::Bool => Some((ty, 0, 1)),
            Type::Int(IntKind::Byte) => Some((ty, 0, 255)),
            Type::Int(IntKind::ShortInt) => Some((ty, -128, 127)),
            Type::Enum(e) => {
                let hi = e.variants.len() as i64 - 1;
                Some((ty, 0, hi))
            }
            Type::Error => None,
            other => {
                self.error(t.span(), format!("{other} is too big to be an array index type"));
                None
            }
        }
    }

    /// Evaluate a constant expression at compile time, if it is one.
    pub(super) fn eval_const(&self, e: &Expr) -> Option<ConstVal> {
        use ConstVal::*;
        match &e.kind {
            ExprKind::Int(n) => Some(Int(*n)),
            ExprKind::Real(x) => Some(Real(*x)),
            ExprKind::Str(s) => Some(Str(s.clone())),
            ExprKind::Name(id) => match &self.a.symbols[self.lookup(&id.key())?].kind {
                SymKind::Const { value, .. } => Some(value.clone()),
                _ => None,
            },
            ExprKind::Unary { op, operand } => match (op, self.eval_const(operand)?) {
                (UnaryOp::Neg, Int(n)) => Some(Int(-n)),
                (UnaryOp::Neg, Real(x)) => Some(Real(-x)),
                (UnaryOp::Plus, v @ (Int(_) | Real(_))) => Some(v),
                (UnaryOp::Not, Bool(b)) => Some(Bool(!b)),
                (UnaryOp::Not, Int(n)) => Some(Int(!n)),
                _ => None,
            },
            ExprKind::Binary { op, lhs, rhs } => {
                let (l, r) = (self.eval_const(lhs)?, self.eval_const(rhs)?);
                match (l, r) {
                    (Int(a), Int(b)) => Some(match op {
                        BinaryOp::Add => Int(a.checked_add(b)?),
                        BinaryOp::Sub => Int(a.checked_sub(b)?),
                        BinaryOp::Mul => Int(a.checked_mul(b)?),
                        BinaryOp::Div => Int(a.checked_div(b)?),
                        BinaryOp::Mod => Int(a.checked_rem(b)?),
                        BinaryOp::RealDiv if b != 0 => Real(a as f64 / b as f64),
                        BinaryOp::And => Int(a & b),
                        BinaryOp::Or => Int(a | b),
                        BinaryOp::Xor => Int(a ^ b),
                        BinaryOp::Shl => Int(a.checked_shl(b as u32)?),
                        BinaryOp::Shr => Int(a.checked_shr(b as u32)?),
                        BinaryOp::Eq => Bool(a == b),
                        BinaryOp::NotEq => Bool(a != b),
                        BinaryOp::Lt => Bool(a < b),
                        BinaryOp::LtEq => Bool(a <= b),
                        BinaryOp::Gt => Bool(a > b),
                        BinaryOp::GtEq => Bool(a >= b),
                        _ => return None,
                    }),
                    (Str(a), Str(b)) if *op == BinaryOp::Add => Some(Str([a, b].concat())),
                    (Bool(a), Bool(b)) => match op {
                        BinaryOp::And => Some(Bool(a && b)),
                        BinaryOp::Or => Some(Bool(a || b)),
                        BinaryOp::Xor => Some(Bool(a != b)),
                        _ => None,
                    },
                    (a, b) => {
                        let as_real = |v: &ConstVal| match v {
                            Int(n) => Some(*n as f64),
                            Real(x) => Some(*x),
                            _ => None,
                        };
                        let (a, b) = (as_real(&a)?, as_real(&b)?);
                        match op {
                            BinaryOp::Add => Some(Real(a + b)),
                            BinaryOp::Sub => Some(Real(a - b)),
                            BinaryOp::Mul => Some(Real(a * b)),
                            BinaryOp::RealDiv => Some(Real(a / b)),
                            _ => None,
                        }
                    }
                }
            }
            ExprKind::Call { callee, args } if args.len() == 1 => {
                let ExprKind::Name(id) = &callee.kind else {
                    return None;
                };
                match (id.key().as_str(), self.eval_const(&args[0])?) {
                    ("chr", Int(n)) if (0..=255).contains(&n) => Some(Str(vec![n as u8])),
                    ("ord", Str(s)) if s.len() == 1 => Some(Int(s[0] as i64)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // ----- Statements ---------------------------------------------------------

    fn stmts(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Empty => {}
            StmtKind::Assign { target, value } => {
                let target_ty = self.place(target, true);
                let value_ty = self.expr(value);
                self.check_assignable(&target_ty, &value_ty, value.span);
            }
            StmtKind::Call(e) => self.call_statement(e),
            StmtKind::Compound(body) => self.stmts(body),
            StmtKind::If { cond, then_branch, else_branch } => {
                self.condition(cond);
                self.stmt(then_branch);
                if let Some(e) = else_branch {
                    self.stmt(e);
                }
            }
            StmtKind::While { cond, body } => {
                self.condition(cond);
                self.stmt(body);
            }
            StmtKind::Repeat { body, cond } => {
                self.stmts(body);
                self.condition(cond);
            }
            StmtKind::For { var, start, end, body, .. } => {
                let var_ty = self.for_variable(var, s.span.start);
                for bound in [start, end] {
                    let ty = self.expr(bound);
                    self.check_assignable(&var_ty, &ty, bound.span);
                }
                self.stmt(body);
                self.active_for_vars.pop();
            }
            StmtKind::Case { scrutinee, arms, else_body } => {
                let ty = self.expr(scrutinee);
                if !ty.is_ordinal() && ty != Type::Error {
                    self.error(scrutinee.span, format!("`case` needs an ordinal value, found {ty}"));
                }
                for arm in arms {
                    for label in &arm.labels {
                        let exprs: Vec<&Expr> = match label {
                            CaseLabel::Value(e) => vec![e],
                            CaseLabel::Range(lo, hi) => vec![lo, hi],
                        };
                        for e in exprs {
                            let label_ty = self.expr(e);
                            if let Some(value) = self.eval_const(e) {
                                self.a.case_labels.insert(e.id, value);
                            } else {
                                self.error(e.span, "`case` labels must be constants");
                            }
                            if !ty.accepts(&label_ty) {
                                self.error(e.span, format!("expected a {ty} label, found {label_ty}"));
                            }
                        }
                    }
                    self.stmt(&arm.body);
                }
                if let Some(body) = else_body {
                    self.stmts(body);
                }
            }
            StmtKind::With { records, .. } => {
                self.error(records[0].span, "`with` is not supported yet");
            }
        }
    }

    /// Resolve a `for` loop's control variable and start tracking it.
    fn for_variable(&mut self, var: &Ident, stmt_start: usize) -> Type {
        let Some(sym) = self.lookup(&var.key()) else {
            self.error(var.span, format!("unknown identifier `{}`", var.name));
            self.active_for_vars.push(usize::MAX);
            return Type::Error;
        };
        let SymKind::Var(v) = &self.a.symbols[sym].kind else {
            self.error(var.span, format!("`{}` is not a variable", var.name));
            self.active_for_vars.push(usize::MAX);
            return Type::Error;
        };
        let ty = v.ty.clone();
        if !ty.is_ordinal() {
            self.error(var.span, format!("a `for` loop variable must be an ordinal type, not {ty}"));
        }
        self.a.decls.insert(var.span.start, sym);
        self.for_loops.push(ForLoop { stmt_start, var: sym, proc: self.cur() });
        // Record the write without counting it as a use outside the loop.
        self.active_for_vars.push(sym);
        self.use_var(sym, true);
        ty
    }

    fn condition(&mut self, cond: &Expr) {
        let ty = self.expr(cond);
        if ty != Type::Bool && ty != Type::Error {
            self.error(cond.span, format!("expected a Boolean condition, found {ty}"));
        }
    }

    fn check_assignable(&mut self, target: &Type, value: &Type, span: Span) {
        if !target.accepts(value) {
            self.error(span, format!("cannot assign a {value} value to a {target} variable"));
        }
    }

    /// A procedure call used as a statement.
    fn call_statement(&mut self, e: &Expr) {
        let (callee, args): (&Expr, &[Expr]) = match &e.kind {
            ExprKind::Call { callee, args } => (callee, args),
            ExprKind::Name(_) | ExprKind::Field(..) => (e, &[]),
            _ => {
                self.error(e.span, "expected a procedure call");
                return;
            }
        };
        let ty = self.call(e, callee, args);
        self.a.types.insert(e.id, ty);
    }

    // ----- Expressions ----------------------------------------------------------

    /// Record a use (and possibly a write) of variable `sym` by the current
    /// procedure.
    fn use_var(&mut self, sym: SymId, write: bool) {
        let cur = self.cur();
        let proc = &mut self.a.procs[cur];
        proc.uses.insert(sym);
        if write {
            proc.writes.insert(sym);
        }
        if !self.active_for_vars.contains(&sym) {
            self.used_outside_for.insert(sym);
        }
    }

    /// Check an expression used as a value and return its type.
    pub(super) fn expr(&mut self, e: &Expr) -> Type {
        let ty = self.expr_inner(e);
        self.a.types.insert(e.id, ty.clone());
        ty
    }

    fn expr_inner(&mut self, e: &Expr) -> Type {
        match &e.kind {
            ExprKind::Int(n) => Type::IntLit(*n),
            ExprKind::Real(_) => Type::Real,
            ExprKind::Str(s) => Type::StrLit(s.len()),
            ExprKind::Nil => Type::Nil,
            ExprKind::Name(ident) => {
                let sym = match self.resolve(&ident.key()) {
                    None => {
                        self.error(ident.span, format!("unknown identifier `{}`", ident.name));
                        return Type::Error;
                    }
                    Some(Resolved::SelfField { self_sym, path, ty }) => {
                        self.a.self_fields.insert(e.id, (self_sym, path));
                        self.use_var(self_sym, false);
                        return ty;
                    }
                    Some(Resolved::SelfMethod { .. }) => return self.call(e, e, &[]),
                    Some(Resolved::Sym(sym)) => sym,
                };
                self.a.refs.insert(e.id, sym);
                match self.a.symbols[sym].kind.clone() {
                    SymKind::Var(v) => {
                        self.use_var(sym, false);
                        v.ty
                    }
                    SymKind::Const { ty, .. } => ty,
                    // A bare function name is a call with no arguments.
                    SymKind::Proc(_) | SymKind::Builtin(_) | SymKind::UnitRoutine(_) => self.call(e, e, &[]),
                    SymKind::TypeName(_) => {
                        self.error(ident.span, format!("`{}` is a type, not a value", ident.name));
                        Type::Error
                    }
                }
            }
            ExprKind::Call { callee, args } => self.call(e, callee, args),
            ExprKind::Unary { op, operand } => {
                let ty = self.expr(operand);
                match op {
                    UnaryOp::Neg | UnaryOp::Plus if ty.is_numeric() => match ty {
                        Type::IntLit(n) if *op == UnaryOp::Neg => Type::IntLit(-n),
                        other => other,
                    },
                    UnaryOp::Not if ty == Type::Bool || ty.is_integer() => ty.concrete(),
                    _ if ty == Type::Error => Type::Error,
                    _ => {
                        self.error(e.span, format!("this operator can't be applied to a {ty} value"));
                        Type::Error
                    }
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.expr(lhs);
                let r = self.expr(rhs);
                self.binary_type(e, *op, &l, &r)
            }
            ExprKind::Format { value, .. } => {
                self.error(value.span, "`:width` formatting is only allowed in `Write`, `Writeln` and `Str`");
                Type::Error
            }
            ExprKind::Field(base, field) => {
                let base_ty = self.expr(base);
                // `obj.FullQ` with no parentheses is a method call too.
                if let Type::Object(object) = &base_ty
                    && object.field_path(&field.name).is_none()
                {
                    return self.call(e, e, &[]);
                }
                self.select_field(base_ty, field)
            }
            ExprKind::Index(base, indexes) => {
                let base_ty = self.expr(base);
                self.select_index(base_ty, indexes, e.span)
            }
            ExprKind::Deref(_) | ExprKind::Set(_) | ExprKind::AddrOf(_) => {
                self.error(e.span, "this kind of expression is not supported yet");
                Type::Error
            }
        }
    }

    fn binary_type(&mut self, e: &Expr, op: BinaryOp, l: &Type, r: &Type) -> Type {
        use BinaryOp::*;
        if *l == Type::Error || *r == Type::Error {
            return Type::Error;
        }
        // Two literals: fold, so `maxrows - 1` keeps adapting to its context.
        if let (Type::IntLit(_), Type::IntLit(_)) = (l, r)
            && let Some(ConstVal::Int(v)) = self.eval_const(e)
        {
            return Type::IntLit(v);
        }
        let result = match op {
            Add if l.is_stringish() && r.is_stringish() => Some(Type::Str { cap: 255 }),
            Add | Sub | Mul => numeric_common(l, r),
            RealDiv if l.is_numeric() && r.is_numeric() => Some(Type::Real),
            Div | Mod | Shl | Shr if l.is_integer() && r.is_integer() => int_common(l, r),
            And | Or | Xor if *l == Type::Bool && *r == Type::Bool => Some(Type::Bool),
            And | Or | Xor => int_common(l, r),
            Eq | NotEq | Lt | LtEq | Gt | GtEq => comparable(l, r).then_some(Type::Bool),
            _ => None,
        };
        result.unwrap_or_else(|| {
            self.error(e.span, format!("operator can't combine {l} and {r} values"));
            Type::Error
        })
    }

    /// Check an expression used as an assignment target (or `var` argument)
    /// and return its type. `write` is false for places that are only read
    /// through, which doesn't happen yet.
    fn place(&mut self, e: &Expr, write: bool) -> Type {
        let ty = match &e.kind {
            ExprKind::Name(ident) => match self.resolve(&ident.key()) {
                None => {
                    self.error(ident.span, format!("unknown identifier `{}`", ident.name));
                    Type::Error
                }
                Some(Resolved::SelfField { self_sym, path, ty }) => {
                    self.a.self_fields.insert(e.id, (self_sym, path));
                    self.use_var(self_sym, write);
                    ty
                }
                // Inside a method function, assigning to the method's name
                // sets its result, as for ordinary functions.
                Some(Resolved::SelfMethod { pid, .. })
                    if self.proc_stack.contains(&pid) && self.a.procs[pid].result.is_some() =>
                {
                    let result = self.a.procs[pid].result.unwrap();
                    self.a.refs.insert(e.id, result);
                    self.use_var(result, write);
                    self.a.procs[pid].ret.clone().unwrap()
                }
                Some(Resolved::SelfMethod { .. }) => {
                    self.error(e.span, format!("cannot assign to `{}`", ident.name));
                    Type::Error
                }
                Some(Resolved::Sym(sym)) => match self.a.symbols[sym].kind.clone() {
                    SymKind::Var(v) => {
                        self.a.refs.insert(e.id, sym);
                        self.use_var(sym, write);
                        v.ty
                    }
                    // Assigning to a function's own name sets its result.
                    SymKind::Proc(p) if self.proc_stack.contains(&p) && self.a.procs[p].result.is_some() => {
                        let result = self.a.procs[p].result.unwrap();
                        self.a.refs.insert(e.id, result);
                        self.use_var(result, write);
                        self.a.procs[p].ret.clone().unwrap()
                    }
                    _ => {
                        self.error(e.span, format!("cannot assign to `{}`", ident.name));
                        Type::Error
                    }
                },
            },
            ExprKind::Field(base, field) => {
                let base_ty = self.place(base, write);
                self.select_field(base_ty, field)
            }
            ExprKind::Index(base, indexes) => {
                let base_ty = self.place(base, write);
                self.select_index(base_ty, indexes, e.span)
            }
            _ => {
                self.error(e.span, "this can't be assigned to or passed by `var`");
                Type::Error
            }
        };
        self.a.types.insert(e.id, ty.clone());
        ty
    }

    fn select_field(&mut self, base: Type, field: &Ident) -> Type {
        match &base {
            Type::Object(o) => match o.field_path(&field.name) {
                Some((_, ty)) => ty,
                None => {
                    self.error(field.span, format!("`{}` has no field `{}`", o.rust_name, field.name));
                    Type::Error
                }
            },
            Type::Record(r) => match r.field(&field.name) {
                Some(f) => f.ty.clone(),
                None => {
                    self.error(field.span, format!("`{}` has no field `{}`", r.rust_name, field.name));
                    Type::Error
                }
            },
            Type::Error => Type::Error,
            other => {
                self.error(field.span, format!("a {other} value has no fields"));
                Type::Error
            }
        }
    }

    /// The element type of `base[i, j, ...]`, checking each index.
    fn select_index(&mut self, base: Type, indexes: &[Expr], span: Span) -> Type {
        let mut ty = base;
        for index in indexes {
            let index_ty = self.expr(index);
            ty = match &ty {
                Type::Array(a) => {
                    if !a.index.accepts(&index_ty) && !(a.index.is_integer() && index_ty.is_integer()) {
                        self.error(index.span, format!("this array is indexed by {}, not {index_ty}", a.index));
                    }
                    a.elem.clone()
                }
                Type::Str { .. } => {
                    if !index_ty.is_integer() && index_ty != Type::Error {
                        self.error(index.span, format!("a string index must be an integer, not {index_ty}"));
                    }
                    Type::Char
                }
                Type::Error => Type::Error,
                other => {
                    self.error(span, format!("a {other} value can't be indexed"));
                    Type::Error
                }
            };
        }
        ty
    }

    /// The variable at the root of a place expression, e.g. `a` in `a[i].x`.
    fn place_root(&self, e: &Expr) -> Option<SymId> {
        match &e.kind {
            ExprKind::Name(_) => self.a.refs.get(&e.id).copied(),
            ExprKind::Field(inner, _) | ExprKind::Index(inner, _) => self.place_root(inner),
            _ => None,
        }
    }

    /// Check a call to a procedure, function, builtin or type cast. `e` is
    /// the whole call expression (used to record its type).
    fn call(&mut self, e: &Expr, callee: &Expr, args: &[Expr]) -> Type {
        // `obj.Method(args)`: a method call on an object.
        if let ExprKind::Field(receiver, method) = &callee.kind {
            return self.method_call(e, callee, receiver, method, args);
        }
        let ExprKind::Name(ident) = &callee.kind else {
            self.error(callee.span, "expected the name of a procedure or function");
            return Type::Error;
        };
        let sym = match self.resolve(&ident.key()) {
            None => {
                self.error(ident.span, format!("unknown identifier `{}`", ident.name));
                return Type::Error;
            }
            // A sibling method called from inside a method: `self.method()`.
            Some(Resolved::SelfMethod { self_sym, pid, path }) => {
                self.a.method_calls.insert(callee.id, (pid, path));
                self.use_var(self_sym, true);
                return self.user_call(e, pid, args);
            }
            Some(Resolved::SelfField { .. }) => {
                self.error(ident.span, format!("`{}` is a field, not a method", ident.name));
                return Type::Error;
            }
            Some(Resolved::Sym(sym)) => sym,
        };
        self.a.refs.insert(callee.id, sym);
        match self.a.symbols[sym].kind.clone() {
            SymKind::Proc(p) => self.user_call(e, p, args),
            SymKind::Builtin(b) => self.builtin_call(e, b, args),
            SymKind::UnitRoutine(r) => self.unit_call(e, r, args),
            SymKind::TypeName(target) => {
                // `Integer(c)` and friends: a Turbo Pascal value typecast.
                if args.len() != 1 {
                    self.error(e.span, "a type cast takes exactly one value");
                    return Type::Error;
                }
                let from = self.expr(&args[0]);
                if !(from.is_ordinal() && target.is_ordinal()) && from != Type::Error {
                    self.error(e.span, format!("cannot cast a {from} value to {target}"));
                }
                target
            }
            SymKind::Var(_) | SymKind::Const { .. } => {
                if !args.is_empty() {
                    self.error(ident.span, format!("`{}` is not a procedure or function", ident.name));
                    return Type::Error;
                }
                self.expr(callee)
            }
        }
    }

    fn unit_call(&mut self, e: &Expr, r: &'static super::units::UnitRoutine, args: &[Expr]) -> Type {
        use super::units::Param;
        if r.params.len() != args.len() {
            self.error(
                e.span,
                format!("`{}` takes {} argument(s) but {} were given", r.name, r.params.len(), args.len()),
            );
        }
        for (param, arg) in r.params.iter().zip(args) {
            match param {
                Param::Int => self.int_arg(arg),
                Param::Str => self.string_arg(arg),
                Param::Real => {
                    let ty = self.expr(arg);
                    if !ty.is_numeric() && ty != Type::Error {
                        self.error(arg.span, format!("expected a number, found {ty}"));
                    }
                }
                Param::VarWord => {
                    let ty = self.place(arg, true);
                    if ty != Type::Int(IntKind::Word) && ty != Type::Error {
                        self.error(arg.span, format!("expected a Word variable, found {ty}"));
                    }
                }
            }
        }
        r.ret.map_or(Type::Void, |ret| ret.ty())
    }

    fn method_call(&mut self, e: &Expr, callee: &Expr, receiver: &Expr, method: &Ident, args: &[Expr]) -> Type {
        // The receiver is passed as `&mut self`, so it counts as a write.
        let receiver_ty = self.place(receiver, true);
        let Type::Object(object) = &receiver_ty else {
            if receiver_ty != Type::Error {
                self.error(method.span, format!("a {receiver_ty} value has no methods"));
            }
            return Type::Error;
        };
        let Some((pid, path)) = self.find_method(object, &method.key()) else {
            self.error(method.span, format!("`{}` has no method `{}`", object.rust_name, method.name));
            return Type::Error;
        };
        self.a.method_calls.insert(callee.id, (pid, path));
        // Remember the receiver like a `var` argument, for the aliasing
        // check: `board.Finish` where `Finish` also uses the global `board`
        // would be two mutable references.
        let receiver_arg = self.place_root(receiver).map(|root| VarArg {
            param: self.a.procs[pid].self_param.as_ref().unwrap().1,
            root,
            whole: matches!(receiver.kind, ExprKind::Name(_)),
            span: receiver.span,
        });
        self.user_call_with(e, pid, args, receiver_arg)
    }

    fn user_call(&mut self, e: &Expr, p: ProcId, args: &[Expr]) -> Type {
        self.user_call_with(e, p, args, None)
    }

    fn user_call_with(&mut self, e: &Expr, p: ProcId, args: &[Expr], receiver: Option<VarArg>) -> Type {
        let cur = self.cur();
        self.a.procs[cur].calls.insert(p);
        let params = self.a.procs[p].params.clone();
        if params.len() != args.len() {
            let name = self.a.procs[p].name.clone();
            self.error(e.span, format!("`{name}` takes {} argument(s) but {} were given", params.len(), args.len()));
            return self.a.procs[p].ret.clone().unwrap_or(Type::Void);
        }
        let mut var_args: Vec<VarArg> = receiver.into_iter().collect();
        for (&param, arg) in params.iter().zip(args) {
            let Var { ty, role, .. } = self.a.var(param).clone();
            if role == VarRole::Param(ParamMode::Var) {
                let arg_ty = self.place(arg, true);
                if arg_ty != ty && arg_ty != Type::Error && ty != Type::Error {
                    self.error(
                        arg.span,
                        format!("a `var` parameter of type {ty} needs a variable of exactly that type, found {arg_ty}"),
                    );
                }
                if let Some(root) = self.place_root(arg) {
                    var_args.push(VarArg { param, root, whole: matches!(arg.kind, ExprKind::Name(_)), span: arg.span });
                }
            } else {
                let arg_ty = self.expr(arg);
                if !ty.accepts(&arg_ty) {
                    self.error(arg.span, format!("expected a {ty} argument, found {arg_ty}"));
                }
            }
        }
        if !var_args.is_empty() {
            self.call_sites.push(CallSite { caller: cur, callee: p, var_args });
        }
        self.a.procs[p].ret.clone().unwrap_or(Type::Void)
    }

    fn builtin_call(&mut self, e: &Expr, b: Builtin, args: &[Expr]) -> Type {
        use Builtin::*;
        let span = e.span;
        // A small helper for checking argument counts. A closure can't
        // borrow `self` mutably while we also use it, so it's a plain
        // function taking what it needs.
        fn arity(
            errors: &mut Vec<Diagnostic>,
            span: Span,
            n: usize,
            range: std::ops::RangeInclusive<usize>,
            name: &str,
        ) -> bool {
            let ok = range.contains(&n);
            if !ok {
                errors.push(Diagnostic::new(span, format!("wrong number of arguments to `{name}`")));
            }
            ok
        }
        let name = format!("{b:?}");
        match b {
            Write | Writeln => {
                for arg in args {
                    let (value, width, decimals) = match &arg.kind {
                        ExprKind::Format { value, width, decimals } => (&**value, Some(width), decimals.as_ref()),
                        _ => (arg, None, None),
                    };
                    let ty = self.expr(value);
                    self.a.types.insert(arg.id, ty.clone());
                    if !(ty.is_ordinal() || ty.is_numeric() || ty.is_stringish() || ty == Type::Error) {
                        self.error(value.span, format!("cannot write a {ty} value"));
                    }
                    for n in width.into_iter().chain(decimals) {
                        let t = self.expr(n);
                        if !t.is_integer() && t != Type::Error {
                            self.error(n.span, "a field width must be an integer");
                        }
                    }
                }
                Type::Void
            }
            Read | Readln => {
                for arg in args {
                    let ty = self.place(arg, true);
                    if !(ty.is_numeric() || ty == Type::Char || matches!(ty, Type::Str { .. }) || ty == Type::Error) {
                        self.error(arg.span, format!("cannot read a {ty} value"));
                    }
                }
                Type::Void
            }
            Halt => {
                if arity(&mut self.errors, span, args.len(), 0..=1, &name) && args.len() == 1 {
                    self.int_arg(&args[0]);
                }
                Type::Void
            }
            Exit | Break | Continue | Randomize => {
                arity(&mut self.errors, span, args.len(), 0..=0, &name);
                Type::Void
            }
            Pi => {
                arity(&mut self.errors, span, args.len(), 0..=0, &name);
                Type::Real
            }
            Ord => self.unary_builtin(
                span,
                args,
                &name,
                |t| t.is_ordinal(),
                |t| match t {
                    Type::Int(IntKind::LongInt) => Type::Int(IntKind::LongInt),
                    Type::Int(IntKind::Word) => Type::Int(IntKind::Word),
                    _ => Type::Int(IntKind::Integer),
                },
            ),
            Chr => self.unary_builtin(span, args, &name, |t| t.is_integer(), |_| Type::Char),
            Odd => self.unary_builtin(span, args, &name, |t| t.is_integer(), |_| Type::Bool),
            Succ | Pred => self.unary_builtin(span, args, &name, |t| t.is_ordinal(), |t| t.concrete()),
            Hi | Lo => self.unary_builtin(span, args, &name, |t| t.is_integer(), |_| Type::Int(IntKind::Byte)),
            Abs | Sqr => self.unary_builtin(span, args, &name, |t| t.is_numeric(), |t| t.concrete()),
            Sqrt | Sin | Cos | ArcTan | Ln | Exp | Int | Frac => {
                self.unary_builtin(span, args, &name, |t| t.is_numeric(), |_| Type::Real)
            }
            Trunc | Round => self.unary_builtin(span, args, &name, |t| t.is_numeric(), |_| Type::Int(IntKind::LongInt)),
            UpCase => {
                self.unary_builtin(span, args, &name, |t| *t == Type::Char || *t == Type::StrLit(1), |_| Type::Char)
            }
            Length => self.unary_builtin(span, args, &name, |t| t.is_stringish(), |_| Type::Int(IntKind::Integer)),
            Random => {
                if arity(&mut self.errors, span, args.len(), 0..=1, &name) && args.len() == 1 {
                    self.int_arg(&args[0]);
                    Type::Int(IntKind::Integer)
                } else {
                    Type::Real
                }
            }
            Inc | Dec => {
                if arity(&mut self.errors, span, args.len(), 1..=2, &name) {
                    let ty = self.place(&args[0], true);
                    if !ty.is_ordinal() && ty != Type::Error {
                        self.error(args[0].span, format!("`{name}` needs an ordinal variable, found {ty}"));
                    }
                    if let Some(n) = args.get(1) {
                        self.int_arg(n);
                    }
                }
                Type::Void
            }
            Copy => {
                if arity(&mut self.errors, span, args.len(), 3..=3, &name) {
                    self.string_arg(&args[0]);
                    self.int_arg(&args[1]);
                    self.int_arg(&args[2]);
                }
                Type::Str { cap: 255 }
            }
            Pos => {
                if arity(&mut self.errors, span, args.len(), 2..=2, &name) {
                    self.string_arg(&args[0]);
                    self.string_arg(&args[1]);
                }
                Type::Int(IntKind::Integer)
            }
            Concat => {
                for arg in args {
                    self.string_arg(arg);
                }
                Type::Str { cap: 255 }
            }
            Delete => {
                if arity(&mut self.errors, span, args.len(), 3..=3, &name) {
                    self.string_place(&args[0]);
                    self.int_arg(&args[1]);
                    self.int_arg(&args[2]);
                }
                Type::Void
            }
            Insert => {
                if arity(&mut self.errors, span, args.len(), 3..=3, &name) {
                    self.string_arg(&args[0]);
                    self.string_place(&args[1]);
                    self.int_arg(&args[2]);
                }
                Type::Void
            }
            Str => {
                if arity(&mut self.errors, span, args.len(), 2..=2, &name) {
                    let (value, width, decimals) = match &args[0].kind {
                        ExprKind::Format { value, width, decimals } => (&**value, Some(width), decimals.as_ref()),
                        _ => (&args[0], None, None),
                    };
                    let ty = self.expr(value);
                    self.a.types.insert(args[0].id, ty.clone());
                    if !ty.is_numeric() && ty != Type::Error {
                        self.error(value.span, format!("`Str` needs a number, found {ty}"));
                    }
                    for n in width.into_iter().chain(decimals) {
                        self.int_arg(n);
                    }
                    self.string_place(&args[1]);
                }
                Type::Void
            }
            Val => {
                if arity(&mut self.errors, span, args.len(), 3..=3, &name) {
                    self.string_arg(&args[0]);
                    let ty = self.place(&args[1], true);
                    if !ty.is_numeric() && ty != Type::Error {
                        self.error(args[1].span, format!("`Val` needs a numeric variable, found {ty}"));
                    }
                    let code = self.place(&args[2], true);
                    if !code.is_integer() && code != Type::Error {
                        self.error(args[2].span, "`Val`'s error code variable must be an integer");
                    }
                }
                Type::Void
            }
        }
    }

    fn unary_builtin(
        &mut self,
        span: Span,
        args: &[Expr],
        name: &str,
        accepts: impl Fn(&Type) -> bool,
        result: impl Fn(&Type) -> Type,
    ) -> Type {
        if args.len() != 1 {
            self.error(span, format!("`{name}` takes exactly one argument"));
            return Type::Error;
        }
        let ty = self.expr(&args[0]);
        if ty == Type::Error {
            return Type::Error;
        }
        if !accepts(&ty) {
            self.error(args[0].span, format!("`{name}` can't be applied to a {ty} value"));
            return Type::Error;
        }
        result(&ty)
    }

    fn int_arg(&mut self, e: &Expr) {
        let ty = self.expr(e);
        if !ty.is_integer() && ty != Type::Error {
            self.error(e.span, format!("expected an integer, found {ty}"));
        }
    }

    fn string_arg(&mut self, e: &Expr) {
        let ty = self.expr(e);
        if !ty.is_stringish() && ty != Type::Error {
            self.error(e.span, format!("expected a string, found {ty}"));
        }
    }

    fn string_place(&mut self, e: &Expr) {
        let ty = self.place(e, true);
        if !matches!(ty, Type::Str { .. } | Type::Error) {
            self.error(e.span, format!("expected a string variable, found {ty}"));
        }
    }
}

/// The type in which `l op r` is computed for `+`, `-` and `*`.
pub fn numeric_common(l: &Type, r: &Type) -> Option<Type> {
    if *l == Type::Real && r.is_numeric() || *r == Type::Real && l.is_numeric() {
        Some(Type::Real)
    } else {
        int_common(l, r)
    }
}

pub fn int_common(l: &Type, r: &Type) -> Option<Type> {
    match (l, r) {
        (Type::Int(a), Type::Int(b)) => Some(Type::Int(IntKind::common(*a, *b))),
        (Type::Int(k), Type::IntLit(v)) | (Type::IntLit(v), Type::Int(k)) => Some(Type::Int(if k.contains(*v) {
            IntKind::common(*k, *k)
        } else {
            IntKind::common(*k, IntKind::for_literal(*v))
        })),
        (Type::IntLit(a), Type::IntLit(b)) => {
            Some(Type::Int(IntKind::common(IntKind::for_literal(*a), IntKind::for_literal(*b))))
        }
        _ => None,
    }
}

/// Can `l` and `r` be compared with `=`, `<` and friends?
fn comparable(l: &Type, r: &Type) -> bool {
    (l.is_numeric() && r.is_numeric())
        || (l.is_stringish() && r.is_stringish())
        || (l == r && l.is_ordinal())
        || (*l == Type::Nil || *r == Type::Nil)
}
