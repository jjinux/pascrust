//! The main sema pass: name resolution and type checking.

use std::collections::{HashMap, HashSet};

use std::rc::Rc;

use super::names::{self, screaming_snake_case, snake_case};
use super::types::{ArrayType, EnumType, Field, ObjectType, PointerType, RecordType, same_ordinal_kind};
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
    /// Pointer types whose target hasn't been resolved yet.
    pending_pointers: Vec<(Rc<PointerType>, Ident)>,
    /// The Rust module name of the unit being checked, if any.
    current_unit: Option<String>,
    /// Each user unit's interface: what a `uses` clause makes visible.
    unit_exports: HashMap<String, HashMap<String, SymId>>,
    /// True while resolving the right-hand side of a `type` declaration.
    declaring_type: bool,
    /// Bounds of type names declared as subranges (`Index = 1..10`).
    subrange_types: HashMap<SymId, (Type, i64, i64)>,
    /// Bounds of variables and parameters declared with a subrange type
    /// (`i: 1..10`), which are stored as their host type: for `High(i)`.
    var_bounds: HashMap<SymId, (i64, i64)>,
    /// The records of the `with` statements we're inside, innermost last,
    /// with their types: a name that is one of their fields means that field.
    with_records: Vec<(Expr, Type)>,
    /// For each `with` we're inside: the variables that choose its record
    /// (`i` in `with a[i] do`) and the record's span. The body must not
    /// change them, since the record is written out again at every use.
    with_selectors: Vec<(HashSet<SymId>, Span)>,
    /// Ids for expressions sema makes up (the pointer copies of `with`),
    /// counting down from the top so they never meet the parser's.
    next_synthetic_id: NodeId,
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
            pending_pointers: Vec::new(),
            method_scopes: Vec::new(),
            subrange_types: HashMap::new(),
            var_bounds: HashMap::new(),
            with_records: Vec::new(),
            with_selectors: Vec::new(),
            next_synthetic_id: NodeId::MAX,
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

    /// Pick a name not in `set` by appending a number: `node_2` for
    /// snake_case names, `Node2` for CamelCase (type) names.
    fn unique_in(set: &mut HashSet<String>, base: String) -> String {
        let camel = base.starts_with(|c: char| c.is_ascii_uppercase());
        let mut name = base.clone();
        let mut n = 2;
        while set.contains(&name) {
            name = if camel { format!("{base}{n}") } else { format!("{base}_{n}") };
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
            ("text", Type::Text),
            // Only reached as a value, as in `SizeOf(string)`: in a type,
            // `string` is a keyword the parser handles.
            ("string", Type::Str { cap: 255 }),
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
            // Faked: TP's variable for the floating-point coprocessor; 3 is an
            // 80387, which every modern machine more than matches.
            ("test8087", ConstVal::Int(3), Type::IntLit(3), "3"),
        ];
        for (name, value, ty, rust) in consts {
            let sym = self.add_symbol(name, rust.to_string(), SymKind::Const { value, ty });
            self.scopes[0].insert(name.to_string(), sym);
        }
        // System variables that live in the runtime.
        let runtime_vars = [
            ("exitcode", Type::Int(IntKind::Integer), "exit_code", "set_exit_code"),
            ("randseed", Type::Int(IntKind::LongInt), "rand_seed", "set_rand_seed"),
            ("filemode", Type::Int(IntKind::Byte), "file_mode", "set_file_mode"),
        ];
        for (name, ty, get, set) in runtime_vars {
            let sym = self.add_symbol(name, get.to_string(), SymKind::RuntimeVar { ty, get, set });
            self.scopes[0].insert(name.to_string(), sym);
        }
        for (name, file) in [("input", StdFile::Input), ("output", StdFile::Output)] {
            let sym = self.add_symbol(name, name.to_string(), SymKind::StdText(file));
            self.scopes[0].insert(name.to_string(), sym);
        }
        for &(name, builtin) in Builtin::ALL {
            let sym = self.add_symbol(name, name.to_string(), SymKind::Builtin(builtin));
            self.scopes[0].insert(name.to_string(), sym);
        }
        self.declare_unit_routines(super::units::SYSTEM, &[]);
    }

    /// Make a standard unit's routines and constants visible.
    fn declare_unit_routines(
        &mut self,
        routines: &'static [super::units::UnitRoutine],
        consts: &[super::units::UnitConst],
    ) {
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
            if let Some((routines, groups)) = super::units::unit(&unit.key()) {
                self.declare_unit_routines(routines, &[]);
                for consts in groups {
                    self.declare_unit_routines(&[], consts);
                }
                for &(name, rust, value) in super::units::unit_bools(&unit.key()) {
                    let kind = SymKind::Const { value: ConstVal::Bool(value), ty: Type::Bool };
                    let sym = self.add_symbol(name, rust.to_string(), kind);
                    self.scopes[0].insert(name.to_string(), sym);
                }
                for &(name, rust, value) in super::units::unit_integers(&unit.key()) {
                    let kind = SymKind::Const { value: ConstVal::Int(value), ty: Type::Int(IntKind::Integer) };
                    let sym = self.add_symbol(name, rust.to_string(), kind);
                    self.scopes[0].insert(name.to_string(), sym);
                }
                if unit.key() == "printer" {
                    let sym = self.add_symbol("Lst", "Lst".to_string(), SymKind::StdText(StdFile::Printer));
                    self.scopes[0].insert("lst".to_string(), sym);
                }
                for var in super::units::unit_vars(&unit.key()) {
                    let kind = SymKind::RuntimeVar { ty: var.ty.ty(), get: var.get, set: var.set };
                    let sym = self.add_symbol(var.name, var.get.to_string(), kind);
                    self.scopes[0].insert(var.name.to_string(), sym);
                }
                for (name, ty) in super::units::unit_types(&unit.key()) {
                    let sym = self.add_symbol(name, ty.rust(), SymKind::TypeName(ty));
                    self.scopes[0].insert(name.to_string(), sym);
                }
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
        for (i, decl) in decls.iter().enumerate() {
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
                    let bounds = self.subrange_bounds(&v.ty);
                    for name in &v.names {
                        let sym = self.declare_var(name, ty.clone(), owner, VarRole::Local);
                        self.a.procs[owner].locals.push(sym);
                        if let Some(bounds) = bounds {
                            self.var_bounds.insert(sym, bounds);
                        }
                    }
                }
                Decl::Proc(p) => self.proc_decl(p),
            }
            // Wait for the end of a run of type declarations: in
            // `type NodePtr = ^Node; Node = record ... end;` inside a
            // procedure, `^Node` means the `Node` declared next, even if
            // there's a global `Node` too.
            if !matches!(decls.get(i + 1), Some(Decl::Type(_))) {
                self.resolve_pointers();
            }
        }
    }

    /// Point each pending `^Name` at its target, now that `Name` may have
    /// been declared. Pascal requires that by the end of the `type` section;
    /// we check after each one and report leftovers at the end.
    fn resolve_pointers(&mut self) {
        let pending = std::mem::take(&mut self.pending_pointers);
        for (ptr, target) in pending {
            match self.lookup(&target.key()).map(|s| self.a.symbols[s].kind.clone()) {
                Some(SymKind::TypeName(ty)) => *ptr.target.borrow_mut() = Some(ty),
                _ => self.pending_pointers.push((ptr, target)),
            }
        }
    }

    /// Every procedure declared `forward` (or in a unit interface) in the
    /// current scope must have been given a body.
    fn check_forwards(&mut self) {
        for (_, target) in std::mem::take(&mut self.pending_pointers) {
            self.error(target.span, format!("unknown type `{}`", target.name));
        }
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
            // The literal's type says what the elements are.
            ConstVal::Set(_) => self.expr(e),
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
            if method.is_virtual {
                self.a.virtuals.insert(pid);
            }
            self.a.method_roles.insert(pid, method.role);
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
            let bounds = group.ty.as_ref().and_then(|t| self.subrange_bounds(t));
            for name in &group.names {
                let param = self.declare_var(name, ty.clone(), pid, VarRole::Param(group.mode));
                self.a.procs[pid].params.push(param);
                if let Some(bounds) = bounds {
                    self.var_bounds.insert(param, bounds);
                }
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
            TypeExpr::File(Some(elem), _) => {
                let elem = self.resolve_type(elem);
                self.a.file_types.push(elem.clone());
                Type::File(Rc::new(elem))
            }
            TypeExpr::File(None, span) => {
                self.error(*span, "untyped files are not supported");
                Type::Error
            }
            TypeExpr::Pointer(target, _) => {
                let alias = if self.declaring_type { name.map(|n| self.type_name(n)) } else { None };
                let ptr = Rc::new(PointerType { target_name: target.name.clone(), target: Default::default(), alias });
                // The target may not be declared yet; resolve it when the
                // declarations around it are done.
                self.pending_pointers.push((ptr.clone(), target.clone()));
                let ty = Type::Pointer(ptr);
                if self.declaring_type {
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
            TypeExpr::Set(elem, span) => {
                // TP sets hold up to 256 values, with ordinals 0 to 255.
                let is_subrange = match &**elem {
                    TypeExpr::Subrange { .. } => true,
                    TypeExpr::Named(id) => self.lookup(&id.key()).is_some_and(|s| self.subrange_types.contains_key(&s)),
                    _ => false,
                };
                if !is_subrange {
                    let elem_ty = self.resolve_type(elem);
                    return match &elem_ty {
                        Type::Char | Type::Bool | Type::Int(IntKind::Byte) | Type::Error => Type::Set(Rc::new(elem_ty)),
                        Type::Enum(e) if e.variants.len() <= 256 => Type::Set(Rc::new(elem_ty)),
                        _ => {
                            self.error(
                                *span,
                                format!("a set's elements must have ordinals from 0 to 255, which {elem_ty} doesn't (a subrange like `0..255` would)"),
                            );
                            Type::Error
                        }
                    };
                }
                let Some((elem_ty, lo, hi)) = self.index_type(elem) else { return Type::Error };
                if lo < 0 || hi > 255 {
                    self.error(
                        *span,
                        format!(
                            "a set's elements must have ordinals from 0 to 255, but {elem_ty} goes from {lo} to {hi}"
                        ),
                    );
                    return Type::Error;
                }
                Type::Set(Rc::new(elem_ty))
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
            // The host is `Integer` if both bounds fit; otherwise the lower
            // bound's type, if the upper bound fits it too (`0..70000` mustn't
            // be stored in 16 bits), and `LongInt` if not.
            Type::Int(k) => Type::Int(if IntKind::Integer.contains(l) && IntKind::Integer.contains(h) {
                IntKind::Integer
            } else if k.contains(l) && k.contains(h) {
                k
            } else {
                IntKind::LongInt
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
            // `SizeOf`, `High` and `Low` are constants too.
            ExprKind::Call { callee, args } if args.len() == 1 && self.type_query_builtin(callee).is_some() => {
                let b = self.type_query_builtin(callee)?;
                self.type_query(b, &args[0]).map(|(value, _)| value)
            }
            ExprKind::Set(elems) => {
                let ord = |e: &Expr| match self.eval_const(e)? {
                    Int(n) => u8::try_from(n).ok(),
                    Str(s) if s.len() == 1 => Some(s[0]),
                    Bool(b) => Some(b as u8),
                    _ => None,
                };
                let mut ranges = Vec::new();
                for elem in elems {
                    ranges.push(match elem {
                        SetElem::Single(e) => (ord(e)?, ord(e)?),
                        SetElem::Range(lo, hi) => (ord(lo)?, ord(hi)?),
                    });
                }
                Some(Set(ranges))
            }
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
                    // An enum constant's value is already its ordinal.
                    ("ord", Int(n)) => Some(Int(n)),
                    ("ord", Bool(b)) => Some(Int(b as i64)),
                    ("abs", Int(n)) => Some(Int(n.abs())),
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
            // `RandSeed := 42`: a runtime variable's setter.
            StmtKind::Assign { target, value } if self.runtime_var(target).is_some() => {
                let (sym, ty) = self.runtime_var(target).unwrap();
                self.a.refs.insert(target.id, sym);
                self.a.types.insert(target.id, ty.clone());
                let value_ty = self.expr(value);
                self.check_assignable(&ty, &value_ty, value.span);
            }
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
                        let mut ords = Vec::new();
                        for e in exprs {
                            let label_ty = self.expr(e);
                            if let Some(value) = self.eval_const(e) {
                                // A label the value can never equal (300 for a
                                // Byte): TP rejects it, and Rust couldn't write it.
                                if let (Type::Int(kind), ConstVal::Int(n)) = (&ty, &value)
                                    && !kind.contains(*n)
                                {
                                    self.error(e.span, format!("case label {n} is out of range for {ty}"));
                                }
                                ords.push(match &value {
                                    ConstVal::Int(n) => Some(*n),
                                    ConstVal::Bool(b) => Some(*b as i64),
                                    ConstVal::Str(bytes) if bytes.len() == 1 => Some(bytes[0] as i64),
                                    _ => None,
                                });
                                self.a.case_labels.insert(e.id, value);
                            } else {
                                self.error(e.span, "`case` labels must be constants");
                            }
                            if !ty.accepts(&label_ty) {
                                self.error(e.span, format!("expected a {ty} label, found {label_ty}"));
                            }
                        }
                        if let (CaseLabel::Range(lo, hi), [Some(low), Some(high)]) = (label, ords.as_slice())
                            && low > high
                        {
                            self.error(lo.span.to(hi.span), "the low end of this case range is above its high end");
                        }
                    }
                    self.stmt(&arm.body);
                }
                if let Some(body) = else_body {
                    self.stmts(body);
                }
            }
            StmtKind::With { records, body } => self.with_statement(s, records, body),
        }
    }

    /// `with r1, r2 do body`. We rewrite the body so that every name that is
    /// a field (or method) of a `with` record says so, `x` → `r1.x`, then
    /// check the rewritten statement as usual; codegen emits it too.
    ///
    /// Turbo Pascal works out which record `with p^ do` means once, before
    /// the body runs. Writing `p^.x` at every use would look again, so if
    /// the body might change `p` (it mentions `p`), or the pointer is a
    /// longer path like `p^.next`, we copy the pointer into a new variable
    /// first: `node := p; ... node^.x ...`. (Copying a `Ptr` only copies
    /// the handle; both point to the same record.)
    fn with_statement(&mut self, s: &Stmt, records: &[Expr], body: &Stmt) {
        let mut prelude = Vec::new();
        let mut selectors = HashSet::new();
        let depth = self.with_records.len();
        for record in records {
            // A later record may itself use the fields of an earlier one:
            // `with list, head^ do`.
            let mut record = self.rewrite_expr(record);
            if let Some(pointer) = last_deref_base(&record)
                && (!matches!(pointer.kind, ExprKind::Name(_)) || mentions_name(body, pointer))
            {
                let pointer = pointer.clone();
                let name = self.with_pointer_copy(&pointer, s.span);
                self.next_synthetic_id -= 1;
                let copy = Expr { id: self.next_synthetic_id, kind: ExprKind::Name(name), span: pointer.span };
                prelude
                    .push(Stmt { kind: StmtKind::Assign { target: copy.clone(), value: pointer }, span: record.span });
                record = replace_last_deref_base(&record, copy);
            }
            let ty = self.place(&record, false);
            if !matches!(ty, Type::Record(_) | Type::Object(_) | Type::Error) {
                self.error(record.span, format!("`with` needs a record or object, found {ty}"));
            }
            self.index_variables(&record, &mut selectors);
            self.with_records.push((record, ty));
        }
        self.check_with_for_loops(body);
        let mut stmts = prelude;
        stmts.push(self.rewrite_stmt(body));
        let rewritten = if stmts.len() == 1 {
            stmts.pop().unwrap()
        } else {
            Stmt { kind: StmtKind::Compound(stmts), span: s.span }
        };
        self.with_selectors.push((selectors, records[0].span));
        self.stmt(&rewritten);
        self.with_selectors.pop();
        self.with_records.truncate(depth);
        self.a.with_bodies.insert(s.span.start, rewritten);
    }

    /// A new local variable for `New(PCircle, Init(2))` to make the record
    /// in, named after the record's type (`circle`).
    fn temp_pointer(&mut self, ty: &Type, span: Span) -> (Ident, SymId) {
        let base = match ty {
            Type::Pointer(p) => match p.target() {
                Type::Object(o) => snake_case(o.rust_name.strip_prefix('T').unwrap_or(&o.rust_name)),
                Type::Record(r) => snake_case(&r.rust_name),
                _ => "record".to_string(),
            },
            _ => "record".to_string(),
        };
        let owner = self.cur();
        let rust_name = self.unique_var_name(owner, base);
        let key = format!("new#{}", self.a.symbols.len());
        let sym = self.add_symbol(&key, rust_name, SymKind::Var(Var { ty: ty.clone(), owner, role: VarRole::Local }));
        self.a.procs[owner].locals.push(sym);
        self.scopes.last_mut().unwrap().insert(key.clone(), sym);
        self.use_var(sym, true);
        (Ident { name: key, span }, sym)
    }

    /// A `for` loop inside a `with` can't use one of the record's fields as
    /// its control variable: TP needs a plain variable there.
    fn check_with_for_loops(&mut self, s: &Stmt) {
        let mut bad = Vec::new();
        visit_stmts(s, &mut |s| {
            if let StmtKind::For { var, .. } = &s.kind
                && self.with_member(&var.key()).is_some()
            {
                bad.push(var.clone());
            }
        });
        for var in bad {
            self.error(
                var.span,
                format!("`{}` is a field of the `with` record; a `for` loop needs a plain variable", var.name),
            );
        }
    }

    /// A new local variable that holds the pointer a `with` record is
    /// reached through, named after the record's type (`node`). Its name
    /// isn't a Pascal identifier, so no Pascal code can refer to it.
    fn with_pointer_copy(&mut self, pointer: &Expr, span: Span) -> Ident {
        let ty = self.place_type(pointer).unwrap_or(Type::Error);
        let base = match &ty {
            Type::Pointer(p) => match p.target() {
                Type::Record(r) => snake_case(&r.rust_name),
                Type::Object(o) => snake_case(&o.rust_name),
                _ => "record".to_string(),
            },
            _ => "record".to_string(),
        };
        let owner = self.cur();
        let rust_name = self.unique_var_name(owner, base);
        let key = format!("with#{}", self.a.symbols.len());
        let sym = self.add_symbol(&key, rust_name, SymKind::Var(Var { ty, owner, role: VarRole::Local }));
        self.a.procs[owner].locals.push(sym);
        self.scopes.last_mut().unwrap().insert(key.clone(), sym);
        Ident { name: key, span }
    }

    /// The variables used in the array indexes of a place (`i` in
    /// `a[i].x`), which has already been checked.
    fn index_variables(&self, e: &Expr, out: &mut HashSet<SymId>) {
        match &e.kind {
            ExprKind::Field(base, _) | ExprKind::Deref(base) => self.index_variables(base, out),
            ExprKind::Index(base, indexes) => {
                self.index_variables(base, out);
                for index in indexes {
                    self.names_in(index, out);
                }
            }
            _ => {}
        }
    }

    fn names_in(&self, e: &Expr, out: &mut HashSet<SymId>) {
        if let Some(&sym) = self.a.refs.get(&e.id) {
            out.insert(sym);
        }
        each_child(e, |child| self.names_in(child, out));
    }

    /// The `with` record that supplies the member `key`, innermost first.
    fn with_member(&self, key: &str) -> Option<&Expr> {
        self.with_records.iter().rev().find_map(|(record, ty)| {
            let found = match ty {
                Type::Record(r) => r.field(key).is_some(),
                Type::Object(o) => o.field_path(key).is_some() || self.find_method(o, key).is_some(),
                _ => false,
            };
            found.then_some(record)
        })
    }

    /// A copy of `e` with each name that a `with` record supplies written
    /// out: `x` → `r.x`. The new `Field` node takes over the name's id.
    fn rewrite_expr(&self, e: &Expr) -> Expr {
        let boxed = |e: &Expr| Box::new(self.rewrite_expr(e));
        let kind = match &e.kind {
            ExprKind::Name(ident) => match self.with_member(&ident.key()) {
                Some(record) => ExprKind::Field(Box::new(record.clone()), ident.clone()),
                None => e.kind.clone(),
            },
            ExprKind::Int(_) | ExprKind::Real(_) | ExprKind::Str(_) | ExprKind::Nil => e.kind.clone(),
            ExprKind::Field(base, field) => ExprKind::Field(boxed(base), field.clone()),
            ExprKind::Index(base, indexes) => {
                ExprKind::Index(boxed(base), indexes.iter().map(|i| self.rewrite_expr(i)).collect())
            }
            ExprKind::Deref(base) => ExprKind::Deref(boxed(base)),
            ExprKind::Call { callee, args } => {
                ExprKind::Call { callee: boxed(callee), args: args.iter().map(|a| self.rewrite_expr(a)).collect() }
            }
            ExprKind::Unary { op, operand } => ExprKind::Unary { op: *op, operand: boxed(operand) },
            ExprKind::Binary { op, lhs, rhs } => ExprKind::Binary { op: *op, lhs: boxed(lhs), rhs: boxed(rhs) },
            ExprKind::Set(elems) => ExprKind::Set(
                elems
                    .iter()
                    .map(|elem| match elem {
                        SetElem::Single(e) => SetElem::Single(self.rewrite_expr(e)),
                        SetElem::Range(lo, hi) => SetElem::Range(self.rewrite_expr(lo), self.rewrite_expr(hi)),
                    })
                    .collect(),
            ),
            ExprKind::AddrOf(base) => ExprKind::AddrOf(boxed(base)),
            ExprKind::Format { value, width, decimals } => {
                ExprKind::Format { value: boxed(value), width: boxed(width), decimals: decimals.as_deref().map(boxed) }
            }
        };
        Expr { id: e.id, kind, span: e.span }
    }

    /// [`Checker::rewrite_expr`] for every expression in a statement. A
    /// nested `with`'s body is left alone: it's rewritten when that `with`
    /// is checked, when its own record's fields are known (and win).
    fn rewrite_stmt(&self, s: &Stmt) -> Stmt {
        let e = |e: &Expr| self.rewrite_expr(e);
        let boxed = |s: &Stmt| Box::new(self.rewrite_stmt(s));
        let list = |body: &[Stmt]| body.iter().map(|s| self.rewrite_stmt(s)).collect();
        let kind = match &s.kind {
            StmtKind::Empty => StmtKind::Empty,
            StmtKind::Assign { target, value } => StmtKind::Assign { target: e(target), value: e(value) },
            StmtKind::Call(call) => StmtKind::Call(e(call)),
            StmtKind::Compound(body) => StmtKind::Compound(list(body)),
            StmtKind::If { cond, then_branch, else_branch } => StmtKind::If {
                cond: e(cond),
                then_branch: boxed(then_branch),
                else_branch: else_branch.as_deref().map(boxed),
            },
            StmtKind::While { cond, body } => StmtKind::While { cond: e(cond), body: boxed(body) },
            StmtKind::Repeat { body, cond } => StmtKind::Repeat { body: list(body), cond: e(cond) },
            StmtKind::For { var, start, end, down, body } => {
                StmtKind::For { var: var.clone(), start: e(start), end: e(end), down: *down, body: boxed(body) }
            }
            StmtKind::Case { scrutinee, arms, else_body } => StmtKind::Case {
                scrutinee: e(scrutinee),
                arms: arms
                    .iter()
                    .map(|arm| CaseArm { labels: arm.labels.clone(), body: self.rewrite_stmt(&arm.body) })
                    .collect(),
                else_body: else_body.as_deref().map(list),
            },
            StmtKind::With { records, body } => {
                StmtKind::With { records: records.iter().map(e).collect(), body: body.clone() }
            }
        };
        Stmt { kind, span: s.span }
    }

    /// Check `FillChar(x, count, value)` or `Move(source, dest, count)`.
    /// Supported: the whole of a variable (`count` = `SizeOf(x)`), filled
    /// with 0 or, for strings and arrays of bytes, any value, or copied to
    /// a variable of the same type; and part of a string or byte array,
    /// from an element (`FillChar(s[1], n, ' ')`).
    fn fill_or_move(&mut self, e: &Expr, b: Builtin, args: &[Expr]) {
        let (target, count) = if b == Builtin::FillChar { (&args[0], &args[1]) } else { (&args[1], &args[2]) };
        let ty = self.place(target, true);
        let source_ty = if b == Builtin::Move { self.place(&args[0], false) } else { ty.clone() };
        self.int_arg(count);
        if b == Builtin::FillChar {
            let value_ty = self.expr(&args[2]);
            if !matches!(value_ty.concrete(), Type::Int(_) | Type::Char | Type::Bool | Type::Error) {
                self.error(args[2].span, format!("`FillChar` fills with a byte or a character, not a {value_ty}"));
            }
            if matches!(self.eval_const(&args[2]), Some(ConstVal::Int(0) | ConstVal::Bool(false)))
                || matches!(self.eval_const(&args[2]), Some(ConstVal::Str(s)) if s == [0])
            {
                self.a.fill_zero.insert(callee_id(e));
            }
        }
        if ty == Type::Error || source_ty == Type::Error {
            return;
        }
        // Part of a string or byte array, from an element.
        if b == Builtin::FillChar
            && let ExprKind::Index(base, indexes) = &target.kind
            && indexes.len() == 1
            && self.place_type(base).is_some_and(|t| byte_sequence(&t))
        {
            return;
        }
        let whole = super::types::tp_size(&ty).is_some_and(|size| self.eval_const(count) == Some(ConstVal::Int(size)));
        let ok = whole
            && match b {
                Builtin::Move => source_ty == ty,
                _ => self.a.fill_zero.contains(&callee_id(e)) || byte_sequence(&ty),
            };
        if !ok {
            let supported = if b == Builtin::Move {
                "pascrust supports `Move` only between two whole variables of the same type, with `SizeOf`"
            } else {
                "pascrust supports `FillChar` on a whole variable (with `SizeOf`, filling with 0 unless it's a string or byte array), or on part of a string or byte array"
            };
            self.error(e.span, supported);
        }
    }

    /// The builtin `callee` names, if it's `SizeOf`, `High` or `Low`.
    fn type_query_builtin(&self, callee: &Expr) -> Option<Builtin> {
        let ExprKind::Name(id) = &callee.kind else { return None };
        match self.a.symbols[self.lookup(&id.key())?].kind {
            SymKind::Builtin(b @ (Builtin::SizeOf | Builtin::High | Builtin::Low)) => Some(b),
            _ => None,
        }
    }

    /// If `e` names a runtime variable (`RandSeed`), its symbol and type.
    fn runtime_var(&self, e: &Expr) -> Option<(SymId, Type)> {
        let ExprKind::Name(id) = &e.kind else { return None };
        let Some(Resolved::Sym(sym)) = self.resolve(&id.key()) else { return None };
        match &self.a.symbols[sym].kind {
            SymKind::RuntimeVar { ty, .. } => Some((sym, ty.clone())),
            _ => None,
        }
    }

    /// The bounds of a subrange type expression, written out (`1..10`) or
    /// named (`Digit`).
    fn subrange_bounds(&self, t: &TypeExpr) -> Option<(i64, i64)> {
        let ord = |e: &Expr| match self.eval_const(e)? {
            ConstVal::Int(n) => Some(n),
            ConstVal::Str(s) if s.len() == 1 => Some(i64::from(s[0])),
            _ => None,
        };
        match t {
            TypeExpr::Subrange { lo, hi, .. } => Some((ord(lo)?, ord(hi)?)),
            TypeExpr::Named(id) => self.subrange_types.get(&self.lookup(&id.key())?).map(|&(_, lo, hi)| (lo, hi)),
            _ => None,
        }
    }

    /// `SizeOf(x)`, `High(x)` or `Low(x)`, where `x` is a type or a
    /// variable: the value, and its type.
    pub(super) fn type_query(&self, b: Builtin, arg: &Expr) -> Option<(ConstVal, Type)> {
        // A type name, possibly a subrange (whose bounds matter here)...
        let mut bounds = None;
        let ty = match &arg.kind {
            ExprKind::Name(id) => match self.resolve(&id.key()) {
                Some(Resolved::Sym(sym)) => match &self.a.symbols[sym].kind {
                    SymKind::TypeName(ty) => {
                        bounds = self.subrange_types.get(&sym).map(|&(_, lo, hi)| (lo, hi));
                        Some(ty.clone())
                    }
                    // A variable declared with a subrange type.
                    SymKind::Var(_) => {
                        bounds = self.var_bounds.get(&sym).copied();
                        None
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        // ...or a variable.
        let ty = match ty {
            Some(ty) => ty,
            None => self.place_type(arg)?,
        };
        // An integer answer is typed like an integer literal, so it fits
        // whatever it's compared with or assigned to.
        if b == Builtin::SizeOf {
            let size = super::types::tp_size(&ty)?;
            return Some((ConstVal::Int(size), Type::IntLit(size)));
        }
        let (lo, hi, ty) = match &ty {
            Type::Array(a) => (a.lo, a.hi, a.index.clone()),
            Type::Str { cap } => (0, i64::from(*cap), Type::Int(IntKind::Byte)),
            Type::Int(kind) => {
                let (lo, hi) = bounds.unwrap_or(kind.range());
                (lo, hi, ty.clone())
            }
            Type::Char => (0, 255, Type::Char),
            Type::Bool => return Some((ConstVal::Bool(b == Builtin::High), Type::Bool)),
            Type::Enum(e) => (0, e.variants.len() as i64 - 1, ty.clone()),
            _ => return None,
        };
        let value = if b == Builtin::High { hi } else { lo };
        let ty = if ty.is_integer() { Type::IntLit(value) } else { ty };
        Some((ConstVal::Int(value), ty))
    }

    /// Resolve a `for` loop's control variable and start tracking it.
    fn for_variable(&mut self, var: &Ident, stmt_start: usize) -> Type {
        let Some(sym) = self.lookup(&var.key()) else {
            // A `with` field already has its own error (`check_with_for_loops`).
            if self.with_member(&var.key()).is_none() {
                self.error(var.span, format!("unknown identifier `{}`", var.name));
            }
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
            if let Some((_, span)) = self.with_selectors.iter().find(|(syms, _)| syms.contains(&sym)) {
                let name = self.a.symbols[sym].name.clone();
                self.error(
                    *span,
                    format!(
                        "the `with` body changes `{name}`, which chooses this record; pascrust writes the record out at every use, so it would pick a different one"
                    ),
                );
            }
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
                    SymKind::Const { ty, .. } | SymKind::RuntimeVar { ty, .. } => ty,
                    SymKind::StdText(_) => Type::Text,
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
                        // `-b` for a `Byte` or `Word` is negative, so it needs a
                        // signed type big enough for the result (in Rust, `-`
                        // doesn't even exist for unsigned integers).
                        Type::Int(IntKind::Byte | IntKind::ShortInt) if *op == UnaryOp::Neg => {
                            Type::Int(IntKind::Integer)
                        }
                        Type::Int(IntKind::Word) if *op == UnaryOp::Neg => Type::Int(IntKind::LongInt),
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
                // `inherited Tricks` or `TDog.Tricks`, a function method of
                // an ancestor, called without parentheses.
                if let ExprKind::Name(id) = &base.kind {
                    let ancestor_type = matches!(
                        self.resolve(&id.key()),
                        Some(Resolved::Sym(sym)) if matches!(self.a.symbols[sym].kind, SymKind::TypeName(Type::Object(_)))
                    );
                    if id.key() == "inherited" || ancestor_type {
                        return self.call(e, e, &[]);
                    }
                }
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
            ExprKind::Deref(base) => {
                let base_ty = self.expr(base);
                self.deref_type(base_ty, e.span)
            }
            ExprKind::Set(elems) => {
                // The element type is the first element's; the others must mix with it.
                let mut elem_ty = Type::Void;
                for elem in elems {
                    let values = match elem {
                        SetElem::Single(e) => vec![e],
                        SetElem::Range(lo, hi) => vec![lo, hi],
                    };
                    for value in values {
                        let ty = self.expr(value);
                        if ty == Type::Error {
                            continue;
                        }
                        if !ty.is_ordinal() {
                            self.error(value.span, format!("set elements must be ordinal values, found {ty}"));
                        } else if elem_ty == Type::Void {
                            elem_ty = ty.concrete();
                        } else if !same_ordinal_kind(&elem_ty, &ty) {
                            self.error(value.span, format!("a set of {elem_ty} can't hold a {}", ty.concrete()));
                        }
                    }
                }
                Type::Set(Rc::new(elem_ty))
            }
            ExprKind::AddrOf(_) => {
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
        let result = match (op, l, r) {
            // Union, difference and intersection.
            (Add | Sub | Mul, Type::Set(a), Type::Set(_)) => {
                if **a == Type::Void {
                    Some(r.clone())
                } else {
                    l.accepts(r).then(|| Type::Set(a.clone()))
                }
            }
            (In, _, Type::Set(elem)) => {
                (l.is_ordinal() && (**elem == Type::Void || same_ordinal_kind(l, elem))).then_some(Type::Bool)
            }
            (Eq | NotEq | LtEq | GtEq, Type::Set(_), Type::Set(_)) => {
                (l.accepts(r) || r.accepts(l)).then_some(Type::Bool)
            }
            _ => None,
        };
        let result = result.or_else(|| match op {
            Add if l.is_stringish() && r.is_stringish() => Some(Type::Str { cap: 255 }),
            Add | Sub | Mul => numeric_common(l, r),
            RealDiv if l.is_numeric() && r.is_numeric() => Some(Type::Real),
            // A shift has the type of the number being shifted: the count
            // doesn't widen it. (TP: "the result is of the same type as x".)
            Shl | Shr if l.is_integer() && r.is_integer() => int_common(l, l),
            Div | Mod if l.is_integer() && r.is_integer() => int_common(l, r),
            And | Or | Xor if *l == Type::Bool && *r == Type::Bool => Some(Type::Bool),
            And | Or | Xor => int_common(l, r),
            Eq | NotEq | Lt | LtEq | Gt | GtEq => comparable(l, r).then_some(Type::Bool),
            _ => None,
        });
        result.unwrap_or_else(|| {
            self.error(e.span, format!("operator can't combine {l} and {r} values"));
            Type::Error
        })
    }

    /// Check an expression used as an assignment target (or `var` argument)
    /// and return its type. `write` is false for places that are only read
    /// through.
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
                    // `Writeln(Output, ...)`: the console.
                    SymKind::StdText(_) => {
                        self.a.refs.insert(e.id, sym);
                        Type::Text
                    }
                    SymKind::RuntimeVar { .. } => {
                        self.error(e.span, format!("`{}` can only be read, or set with `:=`", ident.name));
                        Type::Error
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
            // Writing through `p^` changes the record, not the pointer, so
            // the pointer itself is only read.
            ExprKind::Deref(base) => {
                let base_ty = self.expr(base);
                self.deref_type(base_ty, e.span)
            }
            _ => {
                self.error(e.span, "this can't be assigned to or passed by `var`");
                Type::Error
            }
        };
        self.a.types.insert(e.id, ty.clone());
        ty
    }

    fn deref_type(&mut self, base: Type, span: Span) -> Type {
        match base {
            Type::Pointer(p) => p.target(),
            Type::Error => Type::Error,
            other => {
                self.error(span, format!("`^` needs a pointer, found {other}"));
                Type::Error
            }
        }
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
    /// The record fields from a place's root variable down to it, as
    /// [`VarArg::fields`] wants them, and whether the path is complete (no
    /// array index on the way).
    fn place_fields(e: &Expr) -> (Vec<String>, bool) {
        match &e.kind {
            ExprKind::Field(inner, field) => {
                let (mut fields, complete) = Self::place_fields(inner);
                if complete {
                    fields.push(field.key());
                }
                (fields, complete)
            }
            ExprKind::Index(inner, _) | ExprKind::Deref(inner) => (Self::place_fields(inner).0, false),
            _ => (Vec::new(), true),
        }
    }

    /// Is the place reached through a pointer (`q^.x`)?
    fn through_pointer(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Deref(_) => true,
            ExprKind::Field(inner, _) | ExprKind::Index(inner, _) => Self::through_pointer(inner),
            _ => false,
        }
    }

    fn place_root(&self, e: &Expr) -> Option<SymId> {
        match &e.kind {
            ExprKind::Name(_) => self.a.refs.get(&e.id).copied(),
            ExprKind::Field(inner, _) | ExprKind::Index(inner, _) | ExprKind::Deref(inner) => self.place_root(inner),
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
                self.a.self_calls.push((self.cur(), pid));
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
            SymKind::UnitRoutine(r) => self.unit_call(e, r, &ident.name, args),
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
            SymKind::Var(_) | SymKind::Const { .. } | SymKind::RuntimeVar { .. } | SymKind::StdText(_) => {
                if !args.is_empty() {
                    self.error(ident.span, format!("`{}` is not a procedure or function", ident.name));
                    return Type::Error;
                }
                self.expr(callee)
            }
        }
    }

    /// `name` is the routine's name as the program spells it, for messages.
    fn unit_call(&mut self, e: &Expr, r: &'static super::units::UnitRoutine, name: &str, args: &[Expr]) -> Type {
        use super::units::Param;
        if let Some(why) = r.refused {
            self.error(e.span, format!("pascrust can't run `{name}`: {why}"));
            return Type::Error;
        }
        if r.params.len() != args.len() {
            self.error(e.span, format!("`{name}` takes {} argument(s) but {} were given", r.params.len(), args.len()));
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
                Param::Bool => {
                    let ty = self.expr(arg);
                    if ty != Type::Bool && ty != Type::Error {
                        self.error(arg.span, format!("expected a Boolean, found {ty}"));
                    }
                }
                Param::VarWord => {
                    let ty = self.place(arg, true);
                    if ty != Type::Int(IntKind::Word) && ty != Type::Error {
                        self.error(arg.span, format!("expected a Word variable, found {ty}"));
                    }
                }
                Param::FillPattern | Param::VarFillPattern => {
                    let ty = if *param == Param::FillPattern { self.expr(arg) } else { self.place(arg, true) };
                    if ty != super::units::fill_pattern_type() && ty != Type::Error {
                        self.error(arg.span, format!("expected a FillPatternType, found {ty}"));
                    }
                }
                Param::VarStr(min) => {
                    let ty = self.place(arg, true);
                    match ty {
                        Type::Str { cap } if cap < *min => self.error(
                            arg.span,
                            format!("`{name}` needs a string variable that holds {min} characters here; this one holds {cap}"),
                        ),
                        Type::Str { .. } | Type::Error => {}
                        _ => self.error(arg.span, format!("expected a string variable, found {ty}")),
                    }
                }
                Param::VarDateTime => {
                    let ty = self.place(arg, true);
                    if ty != super::units::date_time_type() && ty != Type::Error {
                        self.error(arg.span, format!("expected a DateTime variable, found {ty}"));
                    }
                }
                Param::VarLongInt => {
                    let ty = self.place(arg, true);
                    if ty != Type::Int(IntKind::LongInt) && ty != Type::Error {
                        self.error(arg.span, format!("expected a LongInt variable, found {ty}"));
                    }
                }
                Param::VarFile => {
                    let ty = self.place(arg, true);
                    if !matches!(ty, Type::Text | Type::File(_) | Type::Error) {
                        self.error(arg.span, format!("expected a file variable, found {ty}"));
                    }
                }
                Param::VarSearchRec => {
                    let ty = self.place(arg, true);
                    if ty != super::units::search_rec_type() && ty != Type::Error {
                        self.error(arg.span, format!("expected a SearchRec variable, found {ty}"));
                    }
                }
                Param::VarBool => {
                    let ty = self.place(arg, true);
                    if ty != Type::Bool && ty != Type::Error {
                        self.error(arg.span, format!("expected a Boolean variable, found {ty}"));
                    }
                }
                Param::VarText => {
                    let ty = self.place(arg, true);
                    if ty != Type::Text && ty != Type::Error {
                        self.error(arg.span, format!("expected a Text variable, found {ty}"));
                    }
                }
                Param::VarInteger => {
                    let ty = self.place(arg, true);
                    if ty != Type::Int(IntKind::Integer) && ty != Type::Error {
                        self.error(arg.span, format!("expected an Integer variable, found {ty}"));
                    }
                }
            }
        }
        r.ret.map_or(Type::Void, |ret| ret.ty())
    }

    fn method_call(&mut self, e: &Expr, callee: &Expr, receiver: &Expr, method: &Ident, args: &[Expr]) -> Type {
        // `inherited Draw` and `TShape.Init(x, y)` call one particular
        // version of a method on `Self`.
        if let ExprKind::Name(id) = &receiver.kind {
            if id.key() == "inherited" {
                return self.static_method_call(e, callee, None, method, args);
            }
            if let Some(Resolved::Sym(sym)) = self.resolve(&id.key())
                && let SymKind::TypeName(Type::Object(object)) = &self.a.symbols[sym].kind
            {
                let object = object.clone();
                return self.static_method_call(e, callee, Some(object), method, args);
            }
        }
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
            fields: Self::place_fields(receiver).0,
            indexed: !Self::place_fields(receiver).1,
            through_pointer: Self::through_pointer(receiver),
            span: receiver.span,
        });
        self.user_call_with(e, pid, args, receiver_arg)
    }

    /// `inherited Method(args)` (when `ancestor` is `None`) or
    /// `Ancestor.Method(args)` inside a method: that version of the method,
    /// called on `Self`, with no dispatch.
    fn static_method_call(
        &mut self,
        e: &Expr,
        callee: &Expr,
        ancestor: Option<Rc<ObjectType>>,
        method: &Ident,
        args: &[Expr],
    ) -> Type {
        let Some(&(_, mpid)) = self.method_scopes.last() else {
            self.error(callee.span, "`inherited` and calls like `Parent.Method` only work inside a method");
            return Type::Error;
        };
        let (object, self_sym) = self.a.procs[mpid].self_param.clone().unwrap();
        let ancestor = match ancestor {
            Some(a) => a,
            None => match &object.parent {
                Some(parent) => parent.clone(),
                None => {
                    self.error(callee.span, format!("`{}` has no parent object to inherit from", object.rust_name));
                    return Type::Error;
                }
            },
        };
        let Some((_, path)) = object.lineage().into_iter().find(|(o, _)| *o == ancestor) else {
            self.error(
                callee.span,
                format!("`{}` isn't `{}` or one of its ancestors", ancestor.rust_name, object.rust_name),
            );
            return Type::Error;
        };
        let Some((pid, rest)) = self.find_method(&ancestor, &method.key()) else {
            self.error(method.span, format!("`{}` has no method `{}`", ancestor.rust_name, method.name));
            return Type::Error;
        };
        self.a.method_calls.insert(callee.id, (pid, [path, rest].concat()));
        self.a.static_calls.insert(callee.id);
        self.a.self_calls.push((self.cur(), pid));
        self.use_var(self_sym, true);
        self.user_call(e, pid, args)
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
                // An object may be passed where an ancestor is expected.
                let descendant = match (&arg_ty, &ty) {
                    (Type::Object(arg), Type::Object(param)) => arg.lineage().iter().any(|(o, _)| o == param),
                    _ => false,
                };
                if arg_ty != ty && !descendant && arg_ty != Type::Error && ty != Type::Error {
                    self.error(
                        arg.span,
                        format!("a `var` parameter of type {ty} needs a variable of exactly that type, found {arg_ty}"),
                    );
                }
                if let Some(root) = self.place_root(arg) {
                    var_args.push(VarArg {
                        param,
                        root,
                        whole: matches!(arg.kind, ExprKind::Name(_)),
                        fields: Self::place_fields(arg).0,
                        indexed: !Self::place_fields(arg).1,
                        through_pointer: Self::through_pointer(arg),
                        span: arg.span,
                    });
                }
            } else {
                let arg_ty = self.expr(arg);
                if !ty.accepts(&arg_ty) {
                    self.error(arg.span, format!("expected a {ty} argument, found {arg_ty}"));
                }
            }
        }
        // Two `var` arguments in the same heap record, `q^.x` and `q^.y`,
        // are fine as long as they are different fields.
        let heap: Vec<_> = params
            .iter()
            .zip(args)
            .filter(|(param, _)| self.a.var(**param).role == VarRole::Param(ParamMode::Var))
            .filter_map(|(_, arg)| Some((super::heap_field(arg)?, arg.span)))
            .collect();
        for (i, ((pointer, fields, indexed), span)) in heap.iter().enumerate() {
            let earlier =
                heap[..i].iter().find(|((earlier, f, _), _)| earlier == pointer && super::fields_overlap(f, fields));
            if let Some(((_, earlier_fields, earlier_indexed), _)) = earlier {
                let name = self.a.procs[p].name.clone();
                let why = if *indexed && *earlier_indexed && earlier_fields == fields {
                    " (Rust can't tell that two elements of one array are different)"
                } else {
                    ""
                };
                self.error(
                    *span,
                    format!(
                        "the record `{pointer}^` is passed by `var` more than once in this call to `{name}`; Rust doesn't allow two mutable references to the same variable{why}"
                    ),
                );
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
                // `Write(f, ...)` writes to a file.
                let mut args = args;
                if let Some(first) = args.first()
                    && let Some(file_ty @ (Type::Text | Type::File(_))) = self.place_type(first)
                {
                    self.file_arg(first, &name, false);
                    args = &args[1..];
                    if let Type::File(elem) = file_ty {
                        if b == Writeln {
                            self.error(span, "`Writeln` needs a text file");
                        }
                        for arg in args {
                            let ty = self.expr(arg);
                            if !elem.accepts(&ty) {
                                self.error(arg.span, format!("this file holds {elem} values, not {ty}"));
                            }
                        }
                        return Type::Void;
                    }
                }
                for arg in args {
                    let (value, width, decimals) = match &arg.kind {
                        ExprKind::Format { value, width, decimals } => (&**value, Some(width), decimals.as_ref()),
                        _ => (arg, None, None),
                    };
                    let ty = self.expr(value);
                    self.a.types.insert(arg.id, ty.clone());
                    // Turbo Pascal can't write an enumerated value: the names
                    // don't exist at run time.
                    let writable = (ty.is_ordinal() && !matches!(ty, Type::Enum(_)))
                        || ty.is_numeric()
                        || ty.is_stringish()
                        || ty == Type::Error;
                    if !writable {
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
                let mut args = args;
                if let Some(first) = args.first()
                    && let Some(file_ty @ (Type::Text | Type::File(_))) = self.place_type(first)
                {
                    self.file_arg(first, &name, false);
                    args = &args[1..];
                    if let Type::File(elem) = file_ty {
                        for arg in args {
                            let ty = self.place(arg, true);
                            if ty != *elem && ty != Type::Error {
                                self.error(arg.span, format!("this file holds {elem} values, not {ty}"));
                            }
                        }
                        return Type::Void;
                    }
                }
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
            Assign | Rename => {
                if arity(&mut self.errors, span, args.len(), 2..=2, &name) {
                    self.file_arg(&args[0], &name, false);
                    self.string_arg(&args[1]);
                }
                Type::Void
            }
            Reset | Rewrite | Close | Erase | Flush => {
                if arity(&mut self.errors, span, args.len(), 1..=1, &name) {
                    self.file_arg(&args[0], &name, false);
                }
                Type::Void
            }
            Append => {
                if arity(&mut self.errors, span, args.len(), 1..=1, &name) {
                    self.file_arg(&args[0], &name, true);
                }
                Type::Void
            }
            Truncate => {
                if arity(&mut self.errors, span, args.len(), 1..=1, &name) {
                    self.typed_file_arg(&args[0], &name);
                }
                Type::Void
            }
            Seek => {
                if arity(&mut self.errors, span, args.len(), 2..=2, &name) {
                    self.typed_file_arg(&args[0], &name);
                    self.int_arg(&args[1]);
                }
                Type::Void
            }
            FilePos | FileSize => {
                if arity(&mut self.errors, span, args.len(), 1..=1, &name) {
                    self.typed_file_arg(&args[0], &name);
                }
                Type::Int(IntKind::LongInt)
            }
            Eof | Eoln | SeekEof | SeekEoln => {
                // No argument means standard input.
                if arity(&mut self.errors, span, args.len(), 0..=1, &name) && args.len() == 1 {
                    self.file_arg(&args[0], &name, b != Eof);
                }
                Type::Bool
            }
            IOResult => {
                arity(&mut self.errors, span, args.len(), 0..=0, &name);
                Type::Int(IntKind::Integer)
            }
            Include | Exclude => {
                if arity(&mut self.errors, span, args.len(), 2..=2, &name) {
                    let set_ty = self.place(&args[0], true);
                    let elem_ty = self.expr(&args[1]);
                    match &set_ty {
                        Type::Set(elem) if same_ordinal_kind(elem, &elem_ty) || elem_ty == Type::Error => {}
                        Type::Set(elem) => {
                            self.error(args[1].span, format!("a set of {elem} can't hold a {elem_ty}"));
                        }
                        Type::Error => {}
                        _ => self.error(args[0].span, format!("`{name}` needs a set variable, found {set_ty}")),
                    }
                }
                Type::Void
            }
            TypeOf => {
                if !arity(&mut self.errors, span, args.len(), 1..=1, &name) {
                    return Type::TypeInfo;
                }
                self.a.uses_type_of = true;
                // An object type's name, or an object.
                if let ExprKind::Name(id) = &args[0].kind
                    && let Some(Resolved::Sym(sym)) = self.resolve(&id.key())
                    && let SymKind::TypeName(ty) = &self.a.symbols[sym].kind
                {
                    let ty = ty.clone();
                    self.a.refs.insert(args[0].id, sym);
                    self.a.types.insert(args[0].id, ty.clone());
                    if !matches!(ty, Type::Object(_)) {
                        self.error(args[0].span, format!("`TypeOf` needs an object type, found {ty}"));
                    }
                    return Type::TypeInfo;
                }
                let ty = self.expr(&args[0]);
                if !matches!(ty, Type::Object(_) | Type::Error) {
                    self.error(args[0].span, format!("`TypeOf` needs an object, found {ty}"));
                }
                // `TypeOf(Self)` inside a method (not a procedure nested in one).
                let is_self = self.a.refs.get(&args[0].id).is_some_and(|&s| self.a.symbols[s].rust_name == "self");
                if is_self && self.a.procs[self.cur()].self_param.is_some() {
                    self.a.self_type_of.insert(self.cur());
                }
                Type::TypeInfo
            }
            New | Dispose => {
                if !arity(&mut self.errors, span, args.len(), 1..=2, &name) {
                    return Type::Void;
                }
                // `New(PCircle, Init(2))`: `New` as a function, making a
                // record of the pointer type named.
                let type_arg = match &args[0].kind {
                    ExprKind::Name(id) if b == New => match self.resolve(&id.key()) {
                        Some(Resolved::Sym(sym)) => match &self.a.symbols[sym].kind {
                            SymKind::TypeName(ty @ Type::Pointer(_)) => Some(ty.clone()),
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                };
                let ty = match &type_arg {
                    Some(ty) => ty.clone(),
                    None => self.place(&args[0], true),
                };
                if !matches!(ty, Type::Pointer(_) | Type::Error) {
                    self.error(args[0].span, format!("`{name}` needs a pointer variable, found {ty}"));
                    return Type::Void;
                }
                if let Some(call) = args.get(1) {
                    // The constructor or destructor: a method call on `p^`.
                    let (pointer, temp) = match &type_arg {
                        Some(ty) => {
                            let (name, sym) = self.temp_pointer(ty, span);
                            self.next_synthetic_id -= 1;
                            (
                                Expr { id: self.next_synthetic_id, kind: ExprKind::Name(name), span: args[0].span },
                                Some(sym),
                            )
                        }
                        None => (args[0].clone(), None),
                    };
                    let (method, method_args) = match &call.kind {
                        ExprKind::Call { callee, args } => (callee.as_ref().clone(), args.clone()),
                        _ => (call.clone(), Vec::new()),
                    };
                    let ExprKind::Name(method_name) = &method.kind else {
                        self.error(call.span, format!("expected a constructor or destructor call in `{name}`"));
                        return Type::Void;
                    };
                    let mut id = || {
                        self.next_synthetic_id -= 1;
                        self.next_synthetic_id
                    };
                    let deref = Expr { id: id(), kind: ExprKind::Deref(Box::new(pointer)), span: call.span };
                    let callee =
                        Expr { id: id(), kind: ExprKind::Field(Box::new(deref), method_name.clone()), span: call.span };
                    let call = Expr {
                        id: id(),
                        kind: ExprKind::Call { callee: Box::new(callee), args: method_args },
                        span: call.span,
                    };
                    self.call_statement(&call);
                    // TP: `New` takes a constructor, `Dispose` a destructor.
                    let ExprKind::Call { callee, .. } = &call.kind else { unreachable!() };
                    if let Some(&(pid, _)) = self.a.method_calls.get(&callee.id) {
                        let (wanted, role) = match b {
                            New => ("constructor", MethodRole::Constructor),
                            _ => ("destructor", MethodRole::Destructor),
                        };
                        if self.a.method_roles.get(&pid) != Some(&role) {
                            let method = self.a.procs[pid].name.clone();
                            self.error(call.span, format!("`{name}` needs a {wanted} here, but `{method}` isn't one"));
                        }
                    }
                    self.a.object_calls.insert(callee_id(e), (call, temp));
                } else if type_arg.is_some() {
                    let (_, sym) = self.temp_pointer(&ty, span);
                    let placeholder = Expr { id: e.id, kind: ExprKind::Nil, span };
                    self.a.object_calls.insert(callee_id(e), (placeholder, Some(sym)));
                }
                if type_arg.is_some() { ty } else { Type::Void }
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
            // Worked out now, as TP does: they're constants.
            SizeOf | High | Low => {
                if !arity(&mut self.errors, span, args.len(), 1..=1, &name) {
                    return Type::Error;
                }
                match self.type_query(b, &args[0]) {
                    Some((value, ty)) => {
                        self.a.folded.insert(e.id, value);
                        ty
                    }
                    None => {
                        let wanted =
                            if b == SizeOf { "a variable or a type" } else { "an ordinal type, an array or a string" };
                        self.error(args[0].span, format!("`{name}` needs {wanted}"));
                        Type::Error
                    }
                }
            }
            // Faked: records are freed when nothing points to them, so
            // there's no heap to mark and release.
            Mark | Release => {
                if arity(&mut self.errors, span, args.len(), 1..=1, &name) {
                    let ty = self.place(&args[0], false);
                    if !matches!(ty, Type::Pointer(_) | Type::Error) {
                        self.error(args[0].span, format!("`{name}` needs a pointer variable, found {ty}"));
                    }
                }
                Type::Void
            }
            // Faked: our files are buffered already.
            SetTextBuf => {
                if arity(&mut self.errors, span, args.len(), 2..=3, &name) {
                    // Only read: `SetTextBuf` becomes a comment, so the file
                    // mustn't look changed (that would make it `mut`).
                    let ty = self.place(&args[0], false);
                    if !matches!(ty, Type::Text | Type::Error) {
                        self.error(args[0].span, format!("`SetTextBuf` needs a text file variable, found {ty}"));
                    }
                    self.place(&args[1], false);
                    if let Some(size) = args.get(2) {
                        self.int_arg(size);
                    }
                }
                Type::Void
            }
            GetDir => {
                if arity(&mut self.errors, span, args.len(), 2..=2, &name) {
                    self.int_arg(&args[0]);
                    let ty = self.place(&args[1], true);
                    if !matches!(ty, Type::Str { .. } | Type::Error) {
                        self.error(args[1].span, format!("`GetDir` needs a string variable, found {ty}"));
                    }
                }
                Type::Void
            }
            // TP copies raw bytes; we handle the safe, common cases.
            FillChar | Move => {
                if arity(&mut self.errors, span, args.len(), 3..=3, &name) {
                    self.fill_or_move(e, b, args);
                }
                Type::Void
            }
            Swap => self.unary_builtin(
                span,
                args,
                &name,
                // TP's `Swap(X: Word)` also takes literals up to $FFFF.
                |t| {
                    matches!(t, Type::IntLit(n) if (0..=65535).contains(n))
                        || matches!(t.concrete(), Type::Int(IntKind::Integer | IntKind::Word))
                },
                |t| match t {
                    Type::IntLit(n) if *n > 32767 => Type::Int(IntKind::Word),
                    t => t.concrete(),
                },
            ),
            Assigned => self.unary_builtin(span, args, &name, |t| matches!(t, Type::Pointer(_)), |_| Type::Bool),
            RunError => {
                if arity(&mut self.errors, span, args.len(), 0..=1, &name) && args.len() == 1 {
                    self.int_arg(&args[0]);
                }
                Type::Void
            }
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
                    // TP's `Random(Range: Word): Word`. With a constant range
                    // up to 32767 the result fits an `Integer`, which keeps
                    // the common `Random(6) + 1` free of conversions.
                    match self.eval_const(&args[0]) {
                        Some(ConstVal::Int(n)) if n <= 32767 => Type::Int(IntKind::Integer),
                        _ => {
                            self.a.random_word.insert(callee_id(e));
                            Type::Int(IntKind::Word)
                        }
                    }
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

    /// A file variable argument. File operations change the file variable's
    /// state, so it's treated like a `var` argument.
    fn file_arg(&mut self, e: &Expr, name: &str, text_only: bool) {
        let ty = self.place(e, true);
        // `Input`, `Output` and `Lst` are always open.
        if let Some(&sym) = self.a.refs.get(&e.id)
            && let SymKind::StdText(file) = self.a.symbols[sym].kind
        {
            let reads = matches!(name, "Eof" | "Eoln" | "SeekEof" | "SeekEoln" | "Read" | "Readln");
            match file {
                StdFile::Printer if reads => {
                    self.error(e.span, format!("`Lst` is the printer: `{name}` can't read from it"))
                }
                StdFile::Output if reads => {
                    self.error(e.span, format!("`Output` is for writing: `{name}` can't read from it"))
                }
                StdFile::Input if matches!(name, "Write" | "Writeln") => {
                    self.error(e.span, format!("`Input` is for reading: `{name}` can't write to it"))
                }
                _ if reads || matches!(name, "Write" | "Writeln" | "Flush") => {}
                StdFile::Printer => {
                    self.error(e.span, format!("`Lst` is always open on the printer; `{name}` can't change it"))
                }
                _ => {
                    let which = if file == StdFile::Output { "Output" } else { "Input" };
                    self.error(e.span, format!("pascrust's `{which}` is always the console; `{name}` can't change it"));
                }
            }
        }
        let ok = match ty {
            Type::Text | Type::Error => true,
            Type::File(_) => !text_only,
            _ => false,
        };
        if !ok {
            let wanted = if text_only { "a text file" } else { "a file" };
            self.error(e.span, format!("`{name}` needs {wanted} variable, found {ty}"));
        }
    }

    fn typed_file_arg(&mut self, e: &Expr, name: &str) {
        let ty = self.place(e, true);
        if !matches!(ty, Type::File(_) | Type::Error) {
            self.error(e.span, format!("`{name}` needs a typed file (`file of ...`), found {ty}"));
        }
    }

    /// The type of a place expression, worked out without recording
    /// anything (no uses, no errors). Used to peek at the first argument of
    /// `Write` and `Read`, which is a file if it has a file type.
    fn place_type(&self, e: &Expr) -> Option<Type> {
        match &e.kind {
            ExprKind::Name(id) => match self.resolve(&id.key())? {
                Resolved::Sym(sym) => match &self.a.symbols[sym].kind {
                    SymKind::Var(v) => Some(v.ty.clone()),
                    SymKind::RuntimeVar { ty, .. } => Some(ty.clone()),
                    SymKind::StdText(_) => Some(Type::Text),
                    _ => None,
                },
                Resolved::SelfField { ty, .. } => Some(ty),
                Resolved::SelfMethod { .. } => None,
            },
            ExprKind::Field(base, field) => match self.place_type(base)? {
                Type::Record(r) => r.field(&field.name).map(|f| f.ty.clone()),
                Type::Object(o) => o.field_path(&field.name).map(|(_, ty)| ty),
                _ => None,
            },
            ExprKind::Index(base, indexes) => {
                let mut ty = self.place_type(base)?;
                for _ in indexes {
                    ty = match ty {
                        Type::Array(a) => a.elem.clone(),
                        _ => return None,
                    };
                }
                Some(ty)
            }
            ExprKind::Deref(base) => match self.place_type(base)? {
                Type::Pointer(p) => Some(p.target()),
                _ => None,
            },
            _ => None,
        }
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
        || (l == r && (l.is_ordinal() || matches!(l, Type::Pointer(_))))
        || (matches!(l, Type::Pointer(_) | Type::Nil) && matches!(r, Type::Pointer(_) | Type::Nil))
        || (*l == Type::TypeInfo && *r == Type::TypeInfo)
}

/// The pointer under the last `^` of a place: `p` in `p^.x`, `p^.next` in
/// `p^.next^`. `None` if the place has no `^`.
fn last_deref_base(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        ExprKind::Deref(base) => Some(base),
        ExprKind::Field(base, _) | ExprKind::Index(base, _) => last_deref_base(base),
        _ => None,
    }
}

/// `e` with the pointer under its last `^` replaced by `pointer`.
fn replace_last_deref_base(e: &Expr, pointer: Expr) -> Expr {
    let kind = match &e.kind {
        ExprKind::Deref(_) => ExprKind::Deref(Box::new(pointer)),
        ExprKind::Field(base, field) => {
            ExprKind::Field(Box::new(replace_last_deref_base(base, pointer)), field.clone())
        }
        ExprKind::Index(base, indexes) => {
            ExprKind::Index(Box::new(replace_last_deref_base(base, pointer)), indexes.clone())
        }
        _ => unreachable!("only called on a place with a `^`"),
    };
    Expr { id: e.id, kind, span: e.span }
}

/// Call `f` on each direct subexpression of `e`.
fn each_child(e: &Expr, mut f: impl FnMut(&Expr)) {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Real(_) | ExprKind::Str(_) | ExprKind::Nil | ExprKind::Name(_) => {}
        ExprKind::Field(base, _) | ExprKind::Deref(base) | ExprKind::AddrOf(base) => f(base),
        ExprKind::Unary { operand, .. } => f(operand),
        ExprKind::Index(base, indexes) => {
            f(base);
            indexes.iter().for_each(f);
        }
        ExprKind::Call { callee, args } => {
            f(callee);
            args.iter().for_each(f);
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            f(lhs);
            f(rhs);
        }
        ExprKind::Set(elems) => {
            for elem in elems {
                match elem {
                    SetElem::Single(e) => f(e),
                    SetElem::Range(lo, hi) => {
                        f(lo);
                        f(hi);
                    }
                }
            }
        }
        ExprKind::Format { value, width, decimals } => {
            f(value);
            f(width);
            if let Some(d) = decimals {
                f(d);
            }
        }
    }
}

/// Does the statement mention the variable `pointer` (a `Name`) anywhere?
fn mentions_name(s: &Stmt, pointer: &Expr) -> bool {
    let ExprKind::Name(target) = &pointer.kind else { return true };
    let key = target.key();
    fn in_expr(e: &Expr, key: &str) -> bool {
        if let ExprKind::Name(id) = &e.kind {
            return id.key() == key;
        }
        let mut found = false;
        each_child(e, |child| found |= in_expr(child, key));
        found
    }
    let mut found = false;
    let mut exprs = |e: &Expr| found |= in_expr(e, &key);
    visit_stmt_exprs(s, &mut exprs);
    found
}

/// Call `f` on every expression in a statement, nested statements included.
fn visit_stmt_exprs(s: &Stmt, f: &mut impl FnMut(&Expr)) {
    match &s.kind {
        StmtKind::Empty => {}
        StmtKind::Assign { target, value } => {
            f(target);
            f(value);
        }
        StmtKind::Call(e) => f(e),
        StmtKind::Compound(body) => body.iter().for_each(|s| visit_stmt_exprs(s, f)),
        StmtKind::Repeat { body, cond } => {
            body.iter().for_each(|s| visit_stmt_exprs(s, f));
            f(cond);
        }
        StmtKind::If { cond, then_branch, else_branch } => {
            f(cond);
            visit_stmt_exprs(then_branch, f);
            if let Some(e) = else_branch {
                visit_stmt_exprs(e, f);
            }
        }
        StmtKind::While { cond, body } => {
            f(cond);
            visit_stmt_exprs(body, f);
        }
        StmtKind::For { start, end, body, .. } => {
            f(start);
            f(end);
            visit_stmt_exprs(body, f);
        }
        StmtKind::Case { scrutinee, arms, else_body } => {
            f(scrutinee);
            arms.iter().for_each(|arm| visit_stmt_exprs(&arm.body, f));
            else_body.iter().flatten().for_each(|s| visit_stmt_exprs(s, f));
        }
        StmtKind::With { records, body } => {
            records.iter().for_each(&mut *f);
            visit_stmt_exprs(body, f);
        }
    }
}

/// The id of a call's callee (`New` in `New(p, Init)`), which is also the
/// call's own id for a bare name.
fn callee_id(e: &Expr) -> NodeId {
    match &e.kind {
        ExprKind::Call { callee, .. } => callee.id,
        _ => e.id,
    }
}

/// A string, or an array whose elements are single bytes (`Byte`, `Char`,
/// `ShortInt`, `Boolean`): something `FillChar` can fill byte by byte.
fn byte_sequence(ty: &Type) -> bool {
    match ty {
        Type::Str { .. } => true,
        Type::Array(a) => matches!(a.elem, Type::Int(IntKind::Byte | IntKind::ShortInt) | Type::Char | Type::Bool),
        _ => false,
    }
}

/// Call `f` on a statement and every statement nested in it.
fn visit_stmts(s: &Stmt, f: &mut impl FnMut(&Stmt)) {
    f(s);
    match &s.kind {
        StmtKind::Compound(body) | StmtKind::Repeat { body, .. } => body.iter().for_each(|s| visit_stmts(s, f)),
        StmtKind::If { then_branch, else_branch, .. } => {
            visit_stmts(then_branch, f);
            if let Some(e) = else_branch {
                visit_stmts(e, f);
            }
        }
        StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::With { body, .. } => visit_stmts(body, f),
        StmtKind::Case { arms, else_body, .. } => {
            arms.iter().for_each(|arm| visit_stmts(&arm.body, f));
            else_body.iter().flatten().for_each(|s| visit_stmts(s, f));
        }
        StmtKind::Empty | StmtKind::Assign { .. } | StmtKind::Call(_) => {}
    }
}
