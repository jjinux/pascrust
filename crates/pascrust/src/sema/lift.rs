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

use std::collections::{BTreeSet, HashMap, HashSet};

use super::*;
use crate::diag::Diagnostic;

pub(super) fn lift(a: &mut Analysis) {
    let n = a.procs.len();
    let owner = |a: &Analysis, sym: SymId| a.var(sym).owner;

    // need[p]: outer variables p must receive. write_need[p]: the subset p
    // (or something it calls) modifies.
    let mut need: Vec<BTreeSet<SymId>> = Vec::with_capacity(n);
    let mut write_need: Vec<BTreeSet<SymId>> = Vec::with_capacity(n);
    // A procedure "provides" a variable it owns, or one it receives through
    // a parameter that always stands for it (see `find_param_aliases`).
    let provided =
        |a: &Analysis, p: ProcId, v: SymId| owner(a, v) == p || a.capture_aliases.contains_key(&(p, a.canonical(v)));
    for (p, info) in a.procs.iter().enumerate() {
        need.push(info.uses.iter().copied().filter(|&v| !provided(a, p, v)).collect());
        write_need.push(info.writes.iter().copied().filter(|&v| !provided(a, p, v)).collect());
    }

    let mut changed = true;
    while changed {
        changed = false;
        for p in 0..n {
            for &q in &a.procs[p].calls {
                // Variables q needs that p doesn't own must come from p's
                // own caller, so p needs them too.
                for &v in &need[q].clone() {
                    if !provided(a, p, v) && is_ancestor(a, owner(a, v), p) && need[p].insert(v) {
                        changed = true;
                    }
                }
                for &v in &write_need[q].clone() {
                    if !provided(a, p, v) && is_ancestor(a, owner(a, v), p) && write_need[p].insert(v) {
                        changed = true;
                    }
                }
            }
        }
    }

    // Note which parameter aliases matter: those through which the procedure
    // (or something it calls) actually reaches the outer variable.
    for (&(p, target), &param) in &a.capture_aliases.clone() {
        let reaches = |v: &SymId| owner(a, *v) != p && a.canonical(*v) == target;
        let used = a.procs[p].uses.iter().any(reaches) || a.procs[p].calls.iter().any(|&q| need[q].iter().any(reaches));
        if used {
            a.used_aliases.insert((p, param));
        }
    }

    for p in 0..n {
        // `BTreeSet` iterates in sorted order, so captures come out in
        // declaration order and the generated code is deterministic.
        let captures = need[p]
            .iter()
            .filter(|&&v| is_ancestor(a, owner(a, v), p))
            .map(|&sym| {
                // A method's `self` is never copied: a read-only capture of it
                // is a shared reference, `this: &Queue`.
                let is_self = a.sym(sym).rust_name == "self";
                let mode = if write_need[p].contains(&sym) {
                    CaptureMode::RefMut
                } else if a.var(sym).ty.is_copy() && !is_self {
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
/// something it calls. Inside that procedure, the captured variable gets
/// another name (`global_count`); everywhere else it keeps its own. Two
/// captures can share a name too (the global `count`, passed on, and the
/// parent's local `count`): as in Pascal's scopes, the innermost keeps it.
fn rename_clashing_captures(a: &mut Analysis) {
    // How deeply a procedure is nested: the main program is 0.
    let depth = |a: &Analysis, mut p: ProcId| {
        let mut n = 0;
        while let Some(parent) = a.procs[p].parent {
            n += 1;
            p = parent;
        }
        n
    };
    for p in 0..a.procs.len() {
        let own: HashSet<String> =
            a.procs[p].params.iter().chain(&a.procs[p].locals).map(|&s| a.sym(s).rust_name.clone()).collect();
        let captures = a.procs[p].captures.clone();
        for cap in &captures {
            let name = a.sym(cap.sym).rust_name.clone();
            let owner = a.var(cap.sym).owner;
            // Hidden by another capture of the same name declared further in.
            let hidden = captures.iter().any(|other| {
                other.sym != cap.sym
                    && a.sym(other.sym).rust_name == name
                    && depth(a, a.var(other.sym).owner) > depth(a, owner)
            });
            if own.contains(&name) || hidden {
                let renamed = if owner == MAIN {
                    format!("global_{name}")
                } else {
                    format!("{}_{name}", a.procs[owner].rust_name)
                };
                a.capture_names.insert((p, cap.sym), renamed);
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

/// Find `var` parameters that every call binds to the same variable.
///
/// `life.pas` passes the global `generations` to `Pause` as a `var`
/// parameter, and `Pause` also calls a procedure that reads the global
/// directly. In Rust, `Pause` would then hold `&mut generations` alongside a
/// second reference to it, which the borrow checker forbids. But if *every*
/// call passes that very variable, the parameter and the variable are the
/// same storage, so the procedure can simply use the parameter wherever the
/// Pascal code names the variable. That's exactly Pascal's meaning, with no
/// aliasing left for Rust to object to.
///
/// The binding can be indirect (`examples/database/filesys.pas`): `Editor`
/// receives the global `FileSystem` as a parameter and passes its parameter
/// on to a nested procedure, so every parameter in that chain is the global.
/// The chains are followed to a fixed point.
pub(super) fn find_param_aliases(a: &mut Analysis, sites: &[CallSite]) {
    // `canon[param]` = the variable that parameter always stands for.
    let mut canon: HashMap<SymId, SymId> = HashMap::new();
    let resolve = |canon: &HashMap<SymId, SymId>, mut sym: SymId| {
        while let Some(&next) = canon.get(&sym) {
            sym = next;
        }
        sym
    };
    let mut changed = true;
    while changed {
        changed = false;
        for p in 0..a.procs.len() {
            let calls: Vec<&CallSite> = sites.iter().filter(|s| s.callee == p).collect();
            if calls.is_empty() {
                continue;
            }
            for &param in &a.procs[p].params {
                if canon.contains_key(&param) || a.var(param).role != VarRole::Param(crate::ast::ParamMode::Var) {
                    continue;
                }
                // The variable each call passes for this parameter. A
                // recursive call passing the parameter itself doesn't count.
                let mut bound_to = HashSet::new();
                let mut every_call_binds_whole_variable = true;
                for site in &calls {
                    match site.var_args.iter().find(|arg| arg.param == param) {
                        Some(arg) if arg.whole => {
                            let root = resolve(&canon, arg.root);
                            if root != param {
                                bound_to.insert(root);
                            }
                        }
                        _ => every_call_binds_whole_variable = false,
                    }
                }
                if every_call_binds_whole_variable && bound_to.len() == 1 {
                    canon.insert(param, *bound_to.iter().next().unwrap());
                    changed = true;
                }
            }
        }
    }
    for &param in canon.keys() {
        let p = a.var(param).owner;
        let target = resolve(&canon, param);
        a.capture_aliases.insert((p, target), param);
        a.param_aliases.insert(param, target);
    }
}

/// Reject calls where Rust would see two mutable references to the same
/// variable. Pascal allows this "aliasing" (and its behaviour is subtle even
/// there); Rust's borrow checker forbids it outright.
pub(super) fn check_aliasing(a: &Analysis, sites: &[CallSite]) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    for site in sites {
        let callee = &a.procs[site.callee];
        for (i, arg) in site.var_args.iter().enumerate() {
            let VarArg { root, span, .. } = *arg;
            let name = &a.sym(root).name;
            if let Some(earlier) = site.var_args[..i].iter().find(|earlier| earlier.overlaps(arg)) {
                // `a[1]` and `a[2]` may well be different elements, but Rust
                // can't tell, so say why they count as the same.
                let why = if earlier.variant_bytes && arg.variant_bytes {
                    " (fields of a variant part without a tag field share its bytes, which Rust borrows as a whole; copy one into a local first)"
                } else if earlier.indexed && arg.indexed && earlier.fields == arg.fields {
                    " (Rust can't tell that two elements of one array are different)"
                } else {
                    ""
                };
                errors.push(Diagnostic::new(
                    span,
                    format!(
                        "`{name}` is passed by `var` more than once in this call to `{}`; Rust doesn't allow two mutable references to the same variable{why}",
                        callee.name
                    ),
                ));
            } else if callee
                .captures
                .iter()
                // Through a pointer, the argument only reads the pointer
                // itself, which a callee that reads it too doesn't mind.
                .any(|c| c.sym == root && !(arg.through_pointer && c.mode != CaptureMode::RefMut))
            {
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
