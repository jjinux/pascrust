//! Which `PChar`s could outlive the statement that made them.
//!
//! TP 7 lets a zero-based `array[0..N] of Char` stand for a `PChar` that
//! points into it. PascRust can't point into an ordinary array, so it hands
//! out a copy of the characters instead, for one statement, and writes them
//! back after it (see docs/architecture.md). That's only right if no pointer into
//! the copy survives the statement: `P := A` would leave `P` pointing at a
//! copy that no longer has anything to do with `A`.
//!
//! So the checker records every place a `PChar` value is stored ([`Flow`]) or
//! passed to a routine of the program ([`ArgFlow`]), with what it came from
//! ([`Src`]). From those, [`check`] works out which routines keep a `PChar`
//! parameter (store it somewhere that outlives the call) and which return a
//! pointer into one, following calls until nothing changes (a fixed point,
//! as in `lift`). Then any array that reaches a stored place is an error.

use std::collections::{BTreeSet, HashMap};

use super::{Analysis, ProcId, SymId};
use crate::ast::NodeId;
use crate::diag::{Diagnostic, Span};

/// Where a `PChar` value comes from.
#[derive(Debug, Clone)]
pub enum Src {
    /// A zero-based `Char` array used as a `PChar` (the expression's id).
    Array(NodeId),
    /// A `PChar` (or `Pointer`) variable or parameter.
    Var(SymId),
    /// The result of a call to the program's own function, with where each
    /// `PChar` argument came from.
    Ret(ProcId, Vec<Vec<Src>>),
}

/// Where a `PChar` value goes.
#[derive(Debug, Clone, Copy)]
pub enum Sink {
    /// A local variable, or a value parameter, of the routine: gone when it
    /// returns.
    Local(SymId),
    /// The function's result.
    Result,
    /// Anywhere else: a global, a field, a `var` parameter, the heap.
    Kept,
}

#[derive(Debug, Clone)]
pub struct Flow {
    pub proc: ProcId,
    pub sink: Sink,
    pub srcs: Vec<Src>,
    pub span: Span,
}

/// A `PChar` passed as argument `param` (an index) to the program's own
/// routine `callee`.
#[derive(Debug, Clone)]
pub struct ArgFlow {
    pub caller: ProcId,
    pub callee: ProcId,
    pub param: usize,
    pub srcs: Vec<Src>,
    pub span: Span,
}

/// What a set of sources comes to: parameters of the routine they're in (by
/// index), and arrays.
#[derive(Default)]
struct Resolved {
    params: BTreeSet<usize>,
    arrays: Vec<NodeId>,
}

struct Facts<'a> {
    a: &'a Analysis,
    /// The parameters (by index) each local `PChar` variable may point into.
    locals: HashMap<SymId, BTreeSet<usize>>,
    /// The parameters each function's result may point into.
    returns: HashMap<ProcId, BTreeSet<usize>>,
    /// The parameters each routine keeps.
    keeps: HashMap<ProcId, BTreeSet<usize>>,
}

impl Facts<'_> {
    fn resolve(&self, proc: ProcId, srcs: &[Src], out: &mut Resolved) {
        for src in srcs {
            match src {
                Src::Array(id) => out.arrays.push(*id),
                Src::Var(sym) => {
                    if let Some(i) = self.a.procs[proc].params.iter().position(|p| p == sym) {
                        out.params.insert(i);
                    }
                    if let Some(params) = self.locals.get(sym) {
                        out.params.extend(params);
                    }
                }
                Src::Ret(callee, args) => {
                    for &i in self.returns.get(callee).into_iter().flatten() {
                        if let Some(arg) = args.get(i) {
                            self.resolve(proc, arg, out);
                        }
                    }
                }
            }
        }
    }

    fn resolved(&self, proc: ProcId, srcs: &[Src]) -> Resolved {
        let mut out = Resolved::default();
        self.resolve(proc, srcs, &mut out);
        out
    }

    /// Add `params` to `map[key]`; true if that changed it.
    fn add<K: std::hash::Hash + Eq>(map: &mut HashMap<K, BTreeSet<usize>>, key: K, params: BTreeSet<usize>) -> bool {
        let set = map.entry(key).or_default();
        let before = set.len();
        set.extend(params);
        set.len() != before
    }
}

/// The errors: arrays whose `PChar` would outlive the statement.
pub fn check(a: &Analysis, flows: &[Flow], arg_flows: &[ArgFlow]) -> Vec<Diagnostic> {
    let mut facts = Facts { a, locals: HashMap::new(), returns: HashMap::new(), keeps: HashMap::new() };
    let mut changed = true;
    while changed {
        changed = false;
        for flow in flows {
            let params = facts.resolved(flow.proc, &flow.srcs).params;
            changed |= match flow.sink {
                Sink::Local(sym) => Facts::add(&mut facts.locals, sym, params),
                Sink::Result => Facts::add(&mut facts.returns, flow.proc, params),
                Sink::Kept => Facts::add(&mut facts.keeps, flow.proc, params),
            };
        }
        for arg in arg_flows {
            if facts.keeps.get(&arg.callee).is_some_and(|k| k.contains(&arg.param)) {
                let params = facts.resolved(arg.caller, &arg.srcs).params;
                changed |= Facts::add(&mut facts.keeps, arg.caller, params);
            }
        }
    }
    let mut errors = Vec::new();
    for flow in flows {
        if !facts.resolved(flow.proc, &flow.srcs).arrays.is_empty() {
            errors.push(Diagnostic::new(
                flow.span,
                format!(
                    "this keeps a PChar into a Char array, which PascRust can't do (it lends a PChar to an array only for one statement); use a PChar from StrNew or GetMem instead{}",
                    super::rewrite_link("pchar-into-array")
                ),
            ));
        }
    }
    for arg in arg_flows {
        let kept = facts.keeps.get(&arg.callee).is_some_and(|k| k.contains(&arg.param));
        if kept && !facts.resolved(arg.caller, &arg.srcs).arrays.is_empty() {
            let name = &a.procs[arg.callee].name;
            errors.push(Diagnostic::new(
                arg.span,
                format!(
                    "`{name}` keeps this PChar after it returns, which PascRust can't do for a Char array (it lends a PChar to an array only for one statement); use a PChar from StrNew or GetMem instead{}",
                    super::rewrite_link("pchar-into-array")
                ),
            ));
        }
    }
    errors
}

/// A statement that lends `Char` arrays to `PChar`s: the copy is made
/// before the statement and written back after it, so a routine the
/// statement calls that changes the same array would be overlooked (before
/// the lend) or undone (after it). `at` is the order things happen in.
#[derive(Debug, Clone, Default)]
pub struct LendingStmt {
    pub start: usize,
    pub lends: Vec<(SymId, u32)>,
    pub calls: Vec<(ProcId, NodeId, Span, u32)>,
}

/// After lifting, when each routine's globals are known: a call that may
/// change a lent array before the lend is run first, into a `let` (as if
/// it were its own statement); one after the lend is an error.
pub fn check_lends(a: &mut Analysis) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    for stmt in std::mem::take(&mut a.lending_stmts) {
        let roots: Vec<SymId> = stmt.lends.iter().map(|&(root, _)| root).collect();
        a.lent_roots.entry(stmt.start).or_default().extend(roots.iter().copied());
        for &(callee, id, span, at) in &stmt.calls {
            for &root in &roots {
                if !a.procs[callee].captures.iter().any(|c| c.sym == root) {
                    continue;
                }
                let first_lend = stmt.lends.iter().filter(|(r, _)| *r == root).map(|&(_, at)| at).min().unwrap_or(0);
                if at < first_lend {
                    a.calls_before_lend.insert(id);
                } else {
                    let (routine, array) = (&a.procs[callee].name, &a.sym(root).name);
                    errors.push(Diagnostic::new(
                        span,
                        format!(
                            "`{routine}` may change `{array}`, which this statement uses as a PChar; PascRust lends a PChar a copy of the array for the statement, so the change would be undone after it; call `{routine}` in a statement of its own{}",
                            super::rewrite_link("call-after-lend")
                        ),
                    ));
                }
            }
        }
    }
    errors
}
