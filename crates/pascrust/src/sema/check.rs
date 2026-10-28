//! The main sema pass: name resolution and type checking.

use std::collections::{HashMap, HashSet};

use super::names::{screaming_snake_case, snake_case};
use super::*;
use crate::ast::*;

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
        a.for_loop_list = self
            .for_loops
            .iter()
            .map(|f| (f.stmt_start, f.var, f.proc))
            .collect();
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
        self.a.symbols.push(Symbol {
            name: name.to_string(),
            rust_name,
            kind,
        });
        self.a.symbols.len() - 1
    }

    /// Put `sym` in the innermost scope under `ident`, reporting duplicates.
    fn bind(&mut self, ident: &Ident, sym: SymId) {
        let scope = self.scopes.last_mut().unwrap();
        if scope.insert(ident.key(), sym).is_some() && self.scopes.len() > 1 {
            self.error(
                ident.span,
                format!("`{}` is declared twice in the same scope", ident.name),
            );
        }
        self.a.decls.insert(ident.span.start, sym);
    }

    fn lookup(&self, key: &str) -> Option<SymId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(key).copied())
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
        let sym = self.add_symbol(
            &ident.name,
            rust_name,
            SymKind::Var(Var { ty, owner, role }),
        );
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
            (
                "maxint",
                ConstVal::Int(32767),
                Type::Int(IntKind::Integer),
                "i16::MAX",
            ),
            (
                "maxlongint",
                ConstVal::Int(i32::MAX as i64),
                Type::Int(IntKind::LongInt),
                "i32::MAX",
            ),
        ];
        for (name, value, ty, rust) in consts {
            let sym = self.add_symbol(name, rust.to_string(), SymKind::Const { value, ty });
            self.scopes[0].insert(name.to_string(), sym);
        }
        for &(name, builtin) in Builtin::ALL {
            let sym = self.add_symbol(name, name.to_string(), SymKind::Builtin(builtin));
            self.scopes[0].insert(name.to_string(), sym);
        }
    }

    // ----- Programs and declarations ---------------------------------------

    pub fn program(&mut self, program: &Program) {
        for unit in &program.uses {
            let known = ["crt", "dos", "graph", "printer"];
            if !known.contains(&unit.key().as_str()) {
                self.error(
                    unit.span,
                    format!("unit `{}` is not supported yet", unit.name),
                );
            }
        }
        self.scopes.push(HashMap::new());
        self.block(&program.block);
    }

    fn block(&mut self, block: &Block) {
        for decl in &block.decls {
            match decl {
                Decl::Const(c) => self.const_decl(c),
                Decl::Type(t) => {
                    let ty = self.resolve_type(&t.ty);
                    let sym = self.add_symbol(&t.name.name, ty.rust(), SymKind::TypeName(ty));
                    self.bind(&t.name, sym);
                }
                Decl::Var(v) => {
                    let ty = self.resolve_type(&v.ty);
                    let owner = self.cur();
                    for name in &v.names {
                        let sym = self.declare_var(name, ty.clone(), owner, VarRole::Local);
                        self.a.procs[owner].locals.push(sym);
                    }
                }
                Decl::Proc(p) => self.proc_decl(p),
            }
        }
        for (key, sym) in self.scopes.last().unwrap().clone() {
            if let SymKind::Proc(p) = self.a.symbols[sym].kind
                && !self.a.procs[p].has_body
            {
                let span = self.a.procs[p].span;
                self.error(
                    span,
                    format!("`{key}` was declared `forward` but never defined"),
                );
            }
        }
        self.stmts(&block.body);
    }

    fn const_decl(&mut self, c: &ConstDecl) {
        if let Some(ty_expr) = &c.ty {
            // A typed constant is really an initialized variable. In Turbo
            // Pascal it lives in the data segment even when declared inside a
            // procedure, so it keeps its value between calls. We get the
            // same effect by making it a global, i.e. a local of `main`.
            let ty = self.resolve_type(ty_expr);
            if let ConstValue::Expr(e) = &c.value {
                let value_ty = self.expr(e);
                if !ty.accepts(&value_ty) {
                    self.error(e.span, format!("expected a {ty} value, found {value_ty}"));
                }
            } else {
                self.error(
                    c.name.span,
                    "array and record typed constants are not supported yet",
                );
            }
            let sym = self.declare_var(&c.name, ty, MAIN, VarRole::Local);
            self.a.procs[MAIN].locals.push(sym);
            self.a.typed_consts.insert(sym, c.clone());
            self.use_var(sym, false);
            return;
        }
        let ConstValue::Expr(e) = &c.value else {
            self.error(
                c.name.span,
                "only typed constants can have a list of values",
            );
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
        let rust_name = Self::unique_in(
            &mut self.used_const_names,
            screaming_snake_case(&c.name.name),
        );
        let sym = self.add_symbol(&c.name.name, rust_name, SymKind::Const { value, ty });
        self.bind(&c.name, sym);
    }

    fn proc_decl(&mut self, d: &ProcDecl) {
        let parent = self.cur();
        // A procedure declared `forward` earlier in this scope gets its body
        // now; reuse its ProcInfo (and its parameters).
        let forward = self
            .scopes
            .last()
            .unwrap()
            .get(&d.name.key())
            .and_then(|&sym| match self.a.symbols[sym].kind {
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

    fn new_proc(&mut self, d: &ProcDecl, parent: ProcId) -> ProcId {
        let pid = self.a.procs.len();
        // Nested procedures are named after their parents: `Visit` inside
        // `Report` becomes `report_visit`, so lifted names never collide.
        let base = if parent == MAIN {
            snake_case(&d.name.name)
        } else {
            format!(
                "{}_{}",
                self.a.procs[parent].rust_name,
                snake_case(&d.name.name)
            )
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
            span: d.name.span,
        });
        // Child procedures inherit the names already taken by their parent.
        let inherited = self.used_names[parent].clone();
        self.used_names.push(inherited);
        self.used_names[pid].insert("result".to_string());

        let sym = self.add_symbol(&d.name.name, rust_name, SymKind::Proc(pid));
        self.bind(&d.name, sym);

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
                SymKind::Var(Var {
                    ty: ret,
                    owner: pid,
                    role: VarRole::Result,
                }),
            );
            self.a.procs[pid].result = Some(result);
            self.a.procs[pid].locals.push(result);
        }
        pid
    }

    fn resolve_type(&mut self, t: &TypeExpr) -> Type {
        match t {
            TypeExpr::Named(ident) => {
                match self.lookup(&ident.key()).map(|s| &self.a.symbols[s].kind) {
                    Some(SymKind::TypeName(ty)) => ty.clone(),
                    Some(_) => {
                        self.error(ident.span, format!("`{}` is not a type", ident.name));
                        Type::Error
                    }
                    None => {
                        self.error(ident.span, format!("unknown type `{}`", ident.name));
                        Type::Error
                    }
                }
            }
            TypeExpr::String(None, _) => Type::Str { cap: 255 },
            TypeExpr::String(Some(len), span) => match self.eval_const(len) {
                Some(ConstVal::Int(n)) if (1..=255).contains(&n) => Type::Str { cap: n as u8 },
                _ => {
                    self.error(
                        *span,
                        "a string's maximum length must be a constant from 1 to 255",
                    );
                    Type::Error
                }
            },
            other => {
                self.error(other.span(), "this kind of type is not supported yet");
                Type::Error
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
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
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
            StmtKind::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                let var_ty = self.for_variable(var, s.span.start);
                for bound in [start, end] {
                    let ty = self.expr(bound);
                    self.check_assignable(&var_ty, &ty, bound.span);
                }
                self.stmt(body);
                self.active_for_vars.pop();
            }
            StmtKind::Case {
                scrutinee,
                arms,
                else_body,
            } => {
                let ty = self.expr(scrutinee);
                if !ty.is_ordinal() && ty != Type::Error {
                    self.error(
                        scrutinee.span,
                        format!("`case` needs an ordinal value, found {ty}"),
                    );
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
                                self.error(
                                    e.span,
                                    format!("expected a {ty} label, found {label_ty}"),
                                );
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
            self.error(
                var.span,
                format!("a `for` loop variable must be an ordinal type, not {ty}"),
            );
        }
        self.a.decls.insert(var.span.start, sym);
        self.for_loops.push(ForLoop {
            stmt_start,
            var: sym,
            proc: self.cur(),
        });
        // Record the write without counting it as a use outside the loop.
        self.active_for_vars.push(sym);
        self.use_var(sym, true);
        ty
    }

    fn condition(&mut self, cond: &Expr) {
        let ty = self.expr(cond);
        if ty != Type::Bool && ty != Type::Error {
            self.error(
                cond.span,
                format!("expected a Boolean condition, found {ty}"),
            );
        }
    }

    fn check_assignable(&mut self, target: &Type, value: &Type, span: Span) {
        if !target.accepts(value) {
            self.error(
                span,
                format!("cannot assign a {value} value to a {target} variable"),
            );
        }
    }

    /// A procedure call used as a statement.
    fn call_statement(&mut self, e: &Expr) {
        let (callee, args): (&Expr, &[Expr]) = match &e.kind {
            ExprKind::Call { callee, args } => (callee, args),
            ExprKind::Name(_) => (e, &[]),
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
                let Some(sym) = self.lookup(&ident.key()) else {
                    self.error(ident.span, format!("unknown identifier `{}`", ident.name));
                    return Type::Error;
                };
                self.a.refs.insert(e.id, sym);
                match self.a.symbols[sym].kind.clone() {
                    SymKind::Var(v) => {
                        self.use_var(sym, false);
                        v.ty
                    }
                    SymKind::Const { ty, .. } => ty,
                    // A bare function name is a call with no arguments.
                    SymKind::Proc(_) | SymKind::Builtin(_) => self.call(e, e, &[]),
                    SymKind::TypeName(_) => {
                        self.error(
                            ident.span,
                            format!("`{}` is a type, not a value", ident.name),
                        );
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
                        self.error(
                            e.span,
                            format!("this operator can't be applied to a {ty} value"),
                        );
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
                self.error(
                    value.span,
                    "`:width` formatting is only allowed in `Write`, `Writeln` and `Str`",
                );
                Type::Error
            }
            ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::Deref(_)
            | ExprKind::Set(_)
            | ExprKind::AddrOf(_) => {
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
            ExprKind::Name(ident) => match self.lookup(&ident.key()) {
                None => {
                    self.error(ident.span, format!("unknown identifier `{}`", ident.name));
                    Type::Error
                }
                Some(sym) => match self.a.symbols[sym].kind.clone() {
                    SymKind::Var(v) => {
                        self.a.refs.insert(e.id, sym);
                        self.use_var(sym, write);
                        v.ty
                    }
                    // Assigning to a function's own name sets its result.
                    SymKind::Proc(p)
                        if self.proc_stack.contains(&p) && self.a.procs[p].result.is_some() =>
                    {
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
            _ => {
                self.error(
                    e.span,
                    "this kind of assignment target is not supported yet",
                );
                Type::Error
            }
        };
        self.a.types.insert(e.id, ty.clone());
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
        let ExprKind::Name(ident) = &callee.kind else {
            self.error(callee.span, "expected the name of a procedure or function");
            return Type::Error;
        };
        let Some(sym) = self.lookup(&ident.key()) else {
            self.error(ident.span, format!("unknown identifier `{}`", ident.name));
            return Type::Error;
        };
        self.a.refs.insert(callee.id, sym);
        match self.a.symbols[sym].kind.clone() {
            SymKind::Proc(p) => self.user_call(e, p, args),
            SymKind::Builtin(b) => self.builtin_call(e, b, args),
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
                    self.error(
                        ident.span,
                        format!("`{}` is not a procedure or function", ident.name),
                    );
                    return Type::Error;
                }
                self.expr(callee)
            }
        }
    }

    fn user_call(&mut self, e: &Expr, p: ProcId, args: &[Expr]) -> Type {
        let cur = self.cur();
        self.a.procs[cur].calls.insert(p);
        let params = self.a.procs[p].params.clone();
        if params.len() != args.len() {
            let name = self.a.procs[p].name.clone();
            self.error(
                e.span,
                format!(
                    "`{name}` takes {} argument(s) but {} were given",
                    params.len(),
                    args.len()
                ),
            );
            return self.a.procs[p].ret.clone().unwrap_or(Type::Void);
        }
        let mut var_roots = Vec::new();
        for (&param, arg) in params.iter().zip(args) {
            let Var { ty, role, .. } = self.a.var(param).clone();
            if role == VarRole::Param(ParamMode::Var) {
                let arg_ty = self.place(arg, true);
                if arg_ty != ty && arg_ty != Type::Error && ty != Type::Error {
                    self.error(arg.span, format!("a `var` parameter of type {ty} needs a variable of exactly that type, found {arg_ty}"));
                }
                if let Some(root) = self.place_root(arg) {
                    var_roots.push((root, arg.span));
                }
            } else {
                let arg_ty = self.expr(arg);
                if !ty.accepts(&arg_ty) {
                    self.error(
                        arg.span,
                        format!("expected a {ty} argument, found {arg_ty}"),
                    );
                }
            }
        }
        if !var_roots.is_empty() {
            self.call_sites.push(CallSite {
                caller: cur,
                callee: p,
                var_roots,
            });
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
                errors.push(Diagnostic::new(
                    span,
                    format!("wrong number of arguments to `{name}`"),
                ));
            }
            ok
        }
        let name = format!("{b:?}");
        match b {
            Write | Writeln => {
                for arg in args {
                    let (value, width, decimals) = match &arg.kind {
                        ExprKind::Format {
                            value,
                            width,
                            decimals,
                        } => (&**value, Some(width), decimals.as_ref()),
                        _ => (arg, None, None),
                    };
                    let ty = self.expr(value);
                    self.a.types.insert(arg.id, ty.clone());
                    if !(ty.is_ordinal()
                        || ty.is_numeric()
                        || ty.is_stringish()
                        || ty == Type::Error)
                    {
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
                    if !(ty.is_numeric()
                        || ty == Type::Char
                        || matches!(ty, Type::Str { .. })
                        || ty == Type::Error)
                    {
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
            Succ | Pred => {
                self.unary_builtin(span, args, &name, |t| t.is_ordinal(), |t| t.concrete())
            }
            Hi | Lo => self.unary_builtin(
                span,
                args,
                &name,
                |t| t.is_integer(),
                |_| Type::Int(IntKind::Byte),
            ),
            Abs | Sqr => {
                self.unary_builtin(span, args, &name, |t| t.is_numeric(), |t| t.concrete())
            }
            Sqrt | Sin | Cos | ArcTan | Ln | Exp | Int | Frac => {
                self.unary_builtin(span, args, &name, |t| t.is_numeric(), |_| Type::Real)
            }
            Trunc | Round => self.unary_builtin(
                span,
                args,
                &name,
                |t| t.is_numeric(),
                |_| Type::Int(IntKind::LongInt),
            ),
            UpCase => self.unary_builtin(
                span,
                args,
                &name,
                |t| *t == Type::Char || *t == Type::StrLit(1),
                |_| Type::Char,
            ),
            Length => self.unary_builtin(
                span,
                args,
                &name,
                |t| t.is_stringish(),
                |_| Type::Int(IntKind::Integer),
            ),
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
                        self.error(
                            args[0].span,
                            format!("`{name}` needs an ordinal variable, found {ty}"),
                        );
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
                        ExprKind::Format {
                            value,
                            width,
                            decimals,
                        } => (&**value, Some(width), decimals.as_ref()),
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
                        self.error(
                            args[1].span,
                            format!("`Val` needs a numeric variable, found {ty}"),
                        );
                    }
                    let code = self.place(&args[2], true);
                    if !code.is_integer() && code != Type::Error {
                        self.error(
                            args[2].span,
                            "`Val`'s error code variable must be an integer",
                        );
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
            self.error(
                args[0].span,
                format!("`{name}` can't be applied to a {ty} value"),
            );
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
        (Type::Int(k), Type::IntLit(v)) | (Type::IntLit(v), Type::Int(k)) => {
            Some(Type::Int(if k.contains(*v) {
                IntKind::common(*k, *k)
            } else {
                IntKind::common(*k, IntKind::for_literal(*v))
            }))
        }
        (Type::IntLit(a), Type::IntLit(b)) => Some(Type::Int(IntKind::common(
            IntKind::for_literal(*a),
            IntKind::for_literal(*b),
        ))),
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
