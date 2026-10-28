//! Semantic analysis ("sema"): everything between parsing and codegen.
//!
//! Sema walks the AST once, in source order (Pascal requires declaring
//! things before using them, so one pass is enough), and:
//!
//! 1. resolves every name to a [`Symbol`], honouring Pascal's nested scopes
//!    and case-insensitivity;
//! 2. works out the type of every expression and reports type errors;
//! 3. records which variables each procedure reads, writes and calls.
//!
//! Then it runs the *lifting analysis* (see [`lift`]): Rust has no nested
//! functions that can see their parent's local variables, so every nested
//! procedure becomes a top-level `fn`, and every outer variable it needs
//! (directly, or because something it calls needs it) becomes an extra
//! parameter. Global variables are handled the same way, as locals of
//! `pascal_main` that are passed to the procedures that use them.
//!
//! The results go into an [`Analysis`], a set of side tables keyed by AST
//! node ids that codegen consults.

pub mod builtins;
mod check;
pub use check::{int_common, numeric_common};
mod lift;
pub mod names;
pub mod types;
pub mod units;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::ast::{NodeId, ParamMode, ProcKind, Program, Unit};
use crate::diag::{Diagnostic, Span};
pub use builtins::Builtin;
pub use types::{IntKind, Type};

pub type SymId = usize;
pub type ProcId = usize;

/// The main program body is treated as procedure number 0. Its local
/// variables are the Pascal program's global variables.
pub const MAIN: ProcId = 0;

#[derive(Debug, Clone)]
pub struct Symbol {
    /// The name as spelled at its declaration.
    pub name: String,
    /// The name to use in generated Rust (`MaxCol` becomes `max_col`).
    pub rust_name: String,
    pub kind: SymKind,
}

#[derive(Debug, Clone)]
pub enum SymKind {
    Var(Var),
    Const {
        value: ConstVal,
        ty: Type,
    },
    TypeName(Type),
    Proc(ProcId),
    Builtin(Builtin),
    /// A routine from `Crt`, `Dos` or `System` with a fixed signature.
    UnitRoutine(&'static units::UnitRoutine),
}

#[derive(Debug, Clone)]
pub struct Var {
    pub ty: Type,
    /// The procedure whose Rust function owns this variable.
    pub owner: ProcId,
    pub role: VarRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarRole {
    Local,
    Param(ParamMode),
    /// The implicit result variable of a function: Pascal functions return a
    /// value by assigning to the function's own name.
    Result,
}

/// The value of a constant, computed at compile time.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstVal {
    Int(i64),
    Real(f64),
    Bool(bool),
    Str(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub name: String,
    pub rust_name: String,
    pub kind: ProcKind,
    pub parent: Option<ProcId>,
    pub params: Vec<SymId>,
    pub ret: Option<Type>,
    pub result: Option<SymId>,
    /// Variables declared by this procedure (including the result variable,
    /// but not the parameters).
    pub locals: Vec<SymId>,
    /// Every variable read or written in the body, wherever it's declared.
    pub uses: BTreeSet<SymId>,
    /// Every variable assigned (or passed by `var`) in the body.
    pub writes: BTreeSet<SymId>,
    /// Procedures this one calls directly.
    pub calls: BTreeSet<ProcId>,
    /// Filled in by the lifting analysis: the outer variables this procedure
    /// receives as extra parameters, in a stable order.
    pub captures: Vec<Capture>,
    /// Whether a body has been seen (for `forward` declarations).
    pub has_body: bool,
    /// For a method: the object it belongs to, and the implicit `Self`
    /// parameter (`&mut self` in Rust).
    pub self_param: Option<(std::rc::Rc<types::ObjectType>, SymId)>,
    pub span: Span,
}

/// How a lifted procedure receives one outer variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capture {
    pub sym: SymId,
    pub mode: CaptureMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Read-only and `Copy`: passed by value.
    Value,
    /// Read-only but not `Copy`: passed as `&T`.
    Ref,
    /// Modified by the procedure (or something it calls): passed as `&mut T`.
    RefMut,
}

/// How a variable is reached inside the Rust function currently being
/// generated. This decides whether codegen writes `x` or `*x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// The function owns the variable (`let mut x`) or received it by value.
    Direct,
    /// The function holds a reference (`x: &mut T` or `x: &T`).
    Reference,
}

/// Everything sema learned, for codegen.
#[derive(Debug, Default)]
pub struct Analysis {
    pub symbols: Vec<Symbol>,
    pub procs: Vec<ProcInfo>,
    /// The type of every expression node.
    pub types: HashMap<NodeId, Type>,
    /// The symbol each `Name` expression refers to.
    pub refs: HashMap<NodeId, SymId>,
    /// The symbol introduced by each declared identifier, keyed by the byte
    /// offset where the identifier appears.
    pub decls: HashMap<usize, SymId>,
    /// The procedure declared by each procedure heading, keyed by the byte
    /// offset of its name.
    pub proc_decls: HashMap<usize, ProcId>,
    /// `for` statements (keyed by the statement's start offset) whose loop
    /// variable is only ever used inside that loop. Those become an idiomatic
    /// `for i in a..=b`, with `i` scoped to the loop.
    pub scoped_for_loops: HashSet<usize>,
    /// Enum, record and named array types, in declaration order, for codegen
    /// to emit as Rust items.
    pub type_defs: Vec<(Type, Span)>,
    /// `(procedure, outer variable) → var parameter`: inside that procedure,
    /// the outer variable is reached through the parameter, because every
    /// call passes exactly that variable for it (the key's variable is the
    /// canonical one). See `lift::find_param_aliases`.
    pub capture_aliases: HashMap<(ProcId, SymId), SymId>,
    /// `var` parameter → the variable every call binds it to.
    pub param_aliases: HashMap<SymId, SymId>,
    /// (procedure, parameter) pairs where the procedure relies on the
    /// parameter standing for an outer variable it also uses by name.
    pub used_aliases: HashSet<(ProcId, SymId)>,
    /// Methods by (object type's Rust name, lowercase method name).
    pub methods: HashMap<(String, String), ProcId>,
    /// `Name` expressions that are fields of `Self` inside a method: the
    /// `Self` symbol and the Rust path from it (`["base", "x"]`).
    pub self_fields: HashMap<NodeId, (SymId, Vec<String>)>,
    /// Each `with` statement (by the start of its span) rewritten without
    /// `with`: the record written out in front of every field it supplies,
    /// `with r do x := 1` → `r.x := 1`. Sema checks, and codegen emits, the
    /// rewritten statement.
    pub with_bodies: HashMap<usize, crate::ast::Stmt>,
    /// Method calls: the call's callee expression id → the method, and the
    /// path from the receiver to the object that defines it (`["base"]`
    /// for an inherited method).
    pub method_calls: HashMap<NodeId, (ProcId, Vec<String>)>,
    /// The element type of every `file of T`, so codegen can make those
    /// types storable.
    pub file_types: Vec<Type>,
    /// The user units, in dependency order.
    pub units: Vec<UnitInfo>,
    /// The value of every `case` label, computed at compile time.
    pub case_labels: HashMap<NodeId, ConstVal>,
    /// Variables that need no `let` declaration because they're only used as
    /// scoped loop variables.
    pub elided_vars: HashSet<SymId>,
    /// Variables that are modified somewhere and so need `let mut` (or a
    /// `mut` parameter).
    pub mutated: HashSet<SymId>,
    /// Typed constants (initialized variables), with their declarations so
    /// codegen can emit the initial value.
    pub typed_consts: HashMap<SymId, crate::ast::ConstDecl>,
    /// For variables that are loop variables: whether every use is inside a
    /// `for` that controls it. Internal to sema.
    for_only: BTreeMap<SymId, bool>,
    /// Every `for` loop: (statement start offset, loop variable, procedure).
    for_loop_list: Vec<(usize, SymId, ProcId)>,
}

impl Analysis {
    pub fn sym(&self, id: SymId) -> &Symbol {
        &self.symbols[id]
    }

    pub fn var(&self, id: SymId) -> &Var {
        match &self.symbols[id].kind {
            SymKind::Var(v) => v,
            other => panic!("symbol {id} is not a variable: {other:?}"),
        }
    }

    pub fn type_of(&self, id: NodeId) -> &Type {
        self.types.get(&id).unwrap_or(&Type::Error)
    }

    /// The variable `sym` always stands for: itself, unless it's a `var`
    /// parameter that every call binds to the same outer variable.
    pub fn canonical(&self, sym: SymId) -> SymId {
        self.param_aliases.get(&sym).copied().unwrap_or(sym)
    }

    /// The parameter through which `proc` reaches `sym`, if `sym` is an
    /// outer variable that one of its parameters always stands for.
    fn alias_param(&self, proc: ProcId, sym: SymId) -> Option<SymId> {
        if self.var(sym).owner == proc {
            return None;
        }
        self.capture_aliases.get(&(proc, self.canonical(sym))).copied()
    }

    /// The Rust name for variable `sym` inside procedure `proc`. Usually the
    /// variable's own name, unless the procedure reaches it through an
    /// aliasing `var` parameter (see [`Analysis::capture_aliases`]).
    pub fn var_name(&self, proc: ProcId, sym: SymId) -> &str {
        let sym = self.alias_param(proc, sym).unwrap_or(sym);
        // A method's `self` is called `this` when a nested procedure
        // receives it: only methods may have a parameter named `self`.
        if self.symbols[sym].rust_name == "self" && self.var(sym).owner != proc {
            return "this";
        }
        &self.symbols[sym].rust_name
    }

    /// How variable `sym` is reached from inside procedure `proc`.
    pub fn access(&self, proc: ProcId, sym: SymId) -> Access {
        if self.alias_param(proc, sym).is_some() {
            return Access::Reference;
        }
        let var = self.var(sym);
        if var.owner == proc {
            return match var.role {
                VarRole::Param(ParamMode::Var) => Access::Reference,
                _ => Access::Direct,
            };
        }
        match self.procs[proc].captures.iter().find(|c| c.sym == sym) {
            Some(Capture { mode: CaptureMode::Value, .. }) => Access::Direct,
            Some(_) => Access::Reference,
            None => panic!("procedure {} uses `{}` without capturing it", self.procs[proc].name, self.sym(sym).name),
        }
    }
}

/// Analyze a program and the units it uses. `units` must be in dependency
/// order: each unit after the units it uses.
pub fn analyze(program: &Program, units: &[Unit]) -> Result<Analysis, Vec<Diagnostic>> {
    let mut checker = check::Checker::new();
    checker.program(program, units);
    let (mut analysis, mut errors, call_sites) = checker.finish();
    lift::find_param_aliases(&mut analysis, &call_sites);
    lift::lift(&mut analysis);
    errors.extend(lift::check_aliasing(&analysis, &call_sites));
    if errors.is_empty() { Ok(analysis) } else { Err(errors) }
}

#[derive(Debug, Clone)]
pub struct UnitInfo {
    pub name: String,
    /// The Rust module the unit becomes.
    pub module: String,
    /// The procedure made from the unit's initialization part, if it has one.
    pub init: Option<ProcId>,
    pub span: Span,
}

/// One `var` argument of a call.
#[derive(Debug, Clone)]
pub struct VarArg {
    /// The callee's parameter.
    pub param: SymId,
    /// The variable at the root of the argument (`a` for `a[i].x`).
    pub root: SymId,
    /// True if the argument is the whole variable, not a part of it.
    pub whole: bool,
    /// The record fields from the root to the argument, lowercased:
    /// `["c", "x"]` for `s.c.x`. It stops at the first array index, because
    /// Rust borrows the whole array to index it, so `a[1]` and `a[2]` (and
    /// `r.v[1]` and `r.v[2]`) count as the same place, `[]` and `["v"]`.
    pub fields: Vec<String>,
    /// True if the path has an array index after `fields`.
    pub indexed: bool,
    pub span: Span,
}

/// A place that is a field of a heap record, like `q^.x` or `list^.head^.c.x`:
/// the pointer expression as a key (`q`, `list^.head`) and the fields after
/// its last `^`. `None` for any other place, or if the pointer expression has
/// an array index (we can't tell statically whether two indexes are equal).
///
/// Two `var` arguments into the same heap record must share one borrow of
/// it (one `get_mut()`), so sema checks they don't overlap and codegen
/// borrows the record once for the call.
pub fn heap_field(e: &crate::ast::Expr) -> Option<(String, Vec<String>)> {
    use crate::ast::ExprKind;
    fn key(e: &crate::ast::Expr) -> Option<String> {
        match &e.kind {
            ExprKind::Name(id) => Some(id.key()),
            ExprKind::Field(base, field) => Some(format!("{}.{}", key(base)?, field.key())),
            ExprKind::Deref(base) => Some(format!("{}^", key(base)?)),
            _ => None,
        }
    }
    match &e.kind {
        ExprKind::Deref(base) => Some((key(base)?, Vec::new())),
        ExprKind::Field(base, field) => {
            let (pointer, mut fields) = heap_field(base)?;
            fields.push(field.key());
            Some((pointer, fields))
        }
        _ => None,
    }
}

/// True if one field path starts with the other: `["c"]` and `["c", "x"]`
/// share storage, `["x"]` and `["y"]` don't.
pub fn fields_overlap(a: &[String], b: &[String]) -> bool {
    // `zip` stops at the shorter path, so this checks for a prefix.
    a.iter().zip(b).all(|(a, b)| a == b)
}

impl VarArg {
    /// True if the two arguments could share storage: same root, and one's
    /// field path starts with the other's. `p.x` and `p.y` are disjoint,
    /// and Rust allows `&mut p.x` and `&mut p.y` at the same time.
    pub fn overlaps(&self, other: &VarArg) -> bool {
        self.root == other.root && fields_overlap(&self.fields, &other.fields)
    }
}

/// A call that passes some arguments by `var`, remembered so that aliasing
/// can be checked once the lifting analysis knows what every procedure
/// captures.
#[derive(Debug, Clone)]
pub struct CallSite {
    pub caller: ProcId,
    pub callee: ProcId,
    pub var_args: Vec<VarArg>,
}
