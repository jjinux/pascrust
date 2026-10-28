//! The lifting analysis ("lambda lifting").
//!
//! Pascal procedures can be nested, and a nested procedure can read and
//! write its parents' local variables:
//!
//! ```pascal
//! procedure Report(list: NodePtr);
//! var total: integer;
//!   procedure Visit(p: NodePtr);
//!   begin total := total + p^.value end;   { uses Report's `total` }
//! begin ... end;
//! ```
//!
//! Rust functions can't do that (closures can, but they can't call
//! themselves recursively, and they hold their borrows for as long as they
//! exist). So every procedure becomes a top-level Rust `fn`, and each outer
//! variable it needs becomes an extra parameter:
//! `fn report_visit(total: &mut i16, p: NodePtr)`.
//!
//! "Needs" is transitive: if `A` calls `B` and `B` uses the global `x`, then
//! `A` needs `x` too, to pass it along. We compute this as a *fixed point*:
//! keep propagating needs from callees to callers until nothing changes. That
//! also handles recursion, which a single pass in call order couldn't.

use std::collections::{BTreeSet, HashSet};

use super::*;
use crate::diag::Diagnostic;

pub(super) fn lift(a: &mut Analysis) {
    let n = a.procs.len();
    let owner = |a: &Analysis, sym: SymId| a.var(sym).owner;

    // need[p]: outer variables p must receive. write_need[p]: the subset p
    // (or something it calls) modifies.
    let mut need: Vec<BTreeSet<SymId>> = Vec::with_capacity(n);
    let mut write_need: Vec<BTreeSet<SymId>> = Vec::with_capacity(n);
    for (p, info) in a.procs.iter().enumerate() {
        need.push(info.uses.iter().copied().filter(|&v| owner(a, v) != p).collect());
        write_need.push(info.writes.iter().copied().filter(|&v| owner(a, v) != p).collect());
    }

    let mut changed = true;
    while changed {
        changed = false;
        for p in 0..n {
            for &q in &a.procs[p].calls {
                // Variables q needs that p doesn't own must come from p's
                // own caller, so p needs them too.
                for &v in &need[q].clone() {
                    if owner(a, v) != p && is_ancestor(a, owner(a, v), p) && need[p].insert(v) {
                        changed = true;
                    }
                }
                for &v in &write_need[q].clone() {
                    if owner(a, v) != p && is_ancestor(a, owner(a, v), p) && write_need[p].insert(v) {
                        changed = true;
                    }
                }
            }
        }
    }

    for p in 0..n {
        // `BTreeSet` iterates in sorted order, so captures come out in
        // declaration order and the generated code is deterministic.
        let captures = need[p]
            .iter()
            .filter(|&&v| is_ancestor(a, owner(a, v), p))
            .map(|&sym| {
                let mode = if write_need[p].contains(&sym) {
                    CaptureMode::RefMut
                } else if a.var(sym).ty.is_copy() {
                    CaptureMode::Value
                } else {
                    CaptureMode::Ref
                };
                Capture { sym, mode }
            })
            .collect();
        a.procs[p].captures = captures;
    }

    rename_clashing_captures(a);

    // Which variables need `mut`: those written by their owner, or handed to
    // a callee that writes them.
    for p in 0..n {
        let mut written: BTreeSet<SymId> = a.procs[p].writes.clone();
        for &q in &a.procs[p].calls {
            written.extend(write_need[q].iter().copied());
        }
        for v in written {
            if owner(a, v) == p {
                a.mutated.insert(v);
            }
        }
    }

    // A `for` loop gets an idiomatic, loop-scoped variable when nothing else
    // ever looks at that variable: it's a plain local, used only inside the
    // loops it controls, and no other procedure captures it.
    let captured_anywhere: HashSet<SymId> = need.iter().flatten().copied().collect();
    for &(stmt_start, var, proc) in &a.for_loop_list.clone() {
        let v = a.var(var);
        let scoped = a.for_only.get(&var) == Some(&true)
            && v.owner == proc
            && v.role == VarRole::Local
            && !captured_anywhere.contains(&var);
        if scoped {
            a.scoped_for_loops.insert(stmt_start);
            a.elided_vars.insert(var);
        }
    }
}

/// A lifted procedure receives its captured variables as parameters, so
/// their Rust names must not clash with its own parameters and locals. In
/// Pascal they can't clash (an inner declaration hides the outer one), but a
/// procedure may still need to pass a hidden outer variable through to
/// something it calls. Rename the captured variable everywhere if so.
fn rename_clashing_captures(a: &mut Analysis) {
    for p in 0..a.procs.len() {
        let own: HashSet<String> =
            a.procs[p].params.iter().chain(&a.procs[p].locals).map(|&s| a.sym(s).rust_name.clone()).collect();
        for cap in a.procs[p].captures.clone() {
            let name = a.sym(cap.sym).rust_name.clone();
            if own.contains(&name) {
                let owner = a.var(cap.sym).owner;
                a.symbols[cap.sym].rust_name = if owner == MAIN {
                    format!("global_{name}")
                } else {
                    format!("{}_{name}", a.procs[owner].rust_name)
                };
            }
        }
    }
}

/// Is `ancestor` the same as `p` or one of the procedures enclosing it?
fn is_ancestor(a: &Analysis, ancestor: ProcId, p: ProcId) -> bool {
    let mut cur = Some(p);
    while let Some(c) = cur {
        if c == ancestor {
            return true;
        }
        cur = a.procs[c].parent;
    }
    false
}

/// Find procedures that receive an outer variable twice: once as a
/// captured variable and once through a `var` parameter, because every call
/// passes that very variable by `var`. (`life.pas` does this: `Pause` takes
/// the global `generations` as a `var` parameter, and also calls a procedure
/// that reads the global directly.) Rust forbids holding `&mut generations`
/// alongside a second reference to it. But since the two are always the same
/// variable, the procedure can drop the capture and use the parameter for
/// both. That's exactly Pascal's meaning, with no aliasing left for Rust to
/// object to.
pub(super) fn unify_aliases(a: &mut Analysis, sites: &[CallSite]) {
    for p in 0..a.procs.len() {
        let calls: Vec<&CallSite> = sites.iter().filter(|s| s.callee == p).collect();
        if calls.is_empty() {
            continue;
        }
        for cap in a.procs[p].captures.clone() {
            let var_params = a.procs[p].params.iter().copied().filter(|&param| {
                a.var(param).role == VarRole::Param(crate::ast::ParamMode::Var) && a.var(param).ty == a.var(cap.sym).ty
            });
            // A parameter that every call binds to the captured variable
            // itself (or, for recursive calls, to the parameter again).
            let alias = var_params.into_iter().find(|&param| {
                calls.iter().all(|site| {
                    site.var_args
                        .iter()
                        .any(|arg| arg.param == param && arg.whole && (arg.root == cap.sym || arg.root == param))
                })
            });
            if let Some(param) = alias {
                a.capture_aliases.insert((p, cap.sym), param);
                a.procs[p].captures.retain(|c| c.sym != cap.sym);
            }
        }
    }
}

/// Reject calls where Rust would see two mutable references to the same
/// variable. Pascal allows this "aliasing" (and its behaviour is subtle even
/// there); Rust's borrow checker forbids it outright.
pub(super) fn check_aliasing(a: &Analysis, sites: &[CallSite]) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    for site in sites {
        let callee = &a.procs[site.callee];
        let mut seen = HashSet::new();
        for &VarArg { root, span, .. } in &site.var_args {
            let name = &a.sym(root).name;
            if !seen.insert(root) {
                errors.push(Diagnostic::new(
                    span,
                    format!(
                        "`{name}` is passed by `var` more than once in this call to `{}`; Rust doesn't allow two mutable references to the same variable",
                        callee.name
                    ),
                ));
            } else if callee.captures.iter().any(|c| c.sym == root) {
                errors.push(Diagnostic::new(
                    span,
                    format!(
                        "`{name}` is passed by `var` to `{}`, which also uses `{name}` directly; Rust doesn't allow two references to the same variable when one of them is mutable",
                        callee.name
                    ),
                ));
            }
        }
    }
    errors
}
