//! `label` and `goto`, in the structured uses Rust can express.
//!
//! Rust has no `goto`, but it can leave a block early and repeat a loop:
//!
//! - A jump *forward*, to a label later in the same statement list or an
//!   enclosing one, leaves a labeled block that ends just before the label:
//!   `'done: { ...; break 'done; ... }`. (A labeled block is a loop that
//!   runs once, without writing the loop.)
//! - A jump *back*, to a label earlier in the list, repeats a labeled loop
//!   that starts at the label: `'again: loop { ...; continue 'again; ...;
//!   break 'again; }`.
//!
//! A jump into a statement (`goto Inside` where `Inside:` is in a loop the
//! `goto` isn't in) has no such shape, and is refused. So are jumps whose
//! blocks would cross without one being inside the other.
//!
//! [`plan`] works out the blocks of one statement list; sema checks every
//! list with it, and codegen asks it again where to put the blocks.

use std::collections::HashSet;

use crate::ast::*;
use crate::diag::{Diagnostic, Span};

/// A block or loop a statement list needs for its labels: the statements
/// `lo..hi` (indexes into the list) go inside it.
#[derive(Debug, Clone, PartialEq)]
pub struct GotoBlock {
    /// The label, lowercase (`done`, `10`).
    pub label: String,
    /// The label as written (`Done`), for messages.
    pub name: String,
    /// True for a block that jumps forward (`break`), false for a loop that
    /// jumps back (`continue`).
    pub forward: bool,
    pub lo: usize,
    pub hi: usize,
}

/// Does `s`, or any statement in it, `goto` the label `key`?
fn jumps_to(s: &Stmt, key: &str) -> bool {
    let mut found = false;
    visit(s, &mut |s| found |= matches!(&s.kind, StmtKind::Goto(label) if label.key() == key));
    found
}

/// Call `f` on `s` and every statement nested in it.
fn visit(s: &Stmt, f: &mut impl FnMut(&Stmt)) {
    f(s);
    match &s.kind {
        StmtKind::Compound(body) | StmtKind::Repeat { body, .. } => body.iter().for_each(|s| visit(s, f)),
        StmtKind::If { then_branch, else_branch, .. } => {
            visit(then_branch, f);
            if let Some(e) = else_branch {
                visit(e, f);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::With { body, .. }
        | StmtKind::Labeled { body, .. } => visit(body, f),
        StmtKind::Case { arms, else_body, .. } => {
            arms.iter().for_each(|arm| visit(&arm.body, f));
            else_body.iter().flatten().for_each(|s| visit(s, f));
        }
        StmtKind::Empty | StmtKind::Assign { .. } | StmtKind::Call(_) | StmtKind::Goto(_) => {}
    }
}

/// Which statements of a list can never run: those after a `goto`,
/// `Exit`, `Break` or `Continue`, up to the next label (which a `goto` can
/// reach). Codegen writes them as comments, and sema doesn't check them.
pub fn dead(stmts: &[Stmt]) -> Vec<bool> {
    let mut dead = Vec::with_capacity(stmts.len());
    let mut after_jump = false;
    for s in stmts {
        if !labels_of(s).is_empty() {
            after_jump = false;
        }
        dead.push(after_jump);
        after_jump |= jumps_away(s);
    }
    dead
}

/// True for a statement that always leaves: `goto L`, `Exit`, `Break`,
/// `Continue` (a label on it doesn't change that).
fn jumps_away(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::Goto(_) => true,
        StmtKind::Labeled { body, .. } => jumps_away(body),
        StmtKind::Call(e) => match &e.kind {
            ExprKind::Name(id) => matches!(id.key().as_str(), "exit" | "break" | "continue"),
            _ => false,
        },
        _ => false,
    }
}

/// The blocks a statement list needs for the labels in it, outermost
/// first, or the reason the jumps can't be arranged as blocks.
pub fn plan(stmts: &[Stmt]) -> Result<Vec<GotoBlock>, Diagnostic> {
    let mut blocks = Vec::new();
    // (`L1: L2: x` puts two labels on one statement.)
    for (k, label) in stmts.iter().enumerate().flat_map(|(k, s)| labels_of(s).into_iter().map(move |l| (k, l))) {
        let key = label.key();
        // (A `goto` that never runs needs no block.)
        let dead = dead(stmts);
        let from: Vec<usize> = (0..stmts.len()).filter(|&i| !dead[i] && jumps_to(&stmts[i], &key)).collect();
        // Jumps from before the label go forward, past a block that ends
        // at it; jumps from the label on go back, to a loop that starts
        // at it.
        if let Some(&first) = from.iter().find(|&&i| i < k) {
            blocks.push(GotoBlock { label: key.clone(), name: label.name.clone(), forward: true, lo: first, hi: k });
        }
        if let Some(&last) = from.iter().rev().find(|&&i| i >= k) {
            blocks.push(GotoBlock { label: key, name: label.name.clone(), forward: false, lo: k, hi: last + 1 });
        }
    }
    // Blocks must nest. Where two cross, a block can start earlier (its
    // `goto`s are still inside) and a loop can end later (the statements
    // after its last `goto` just run once more inside it); otherwise the
    // jumps can't be blocks.
    loop {
        let crossing = (0..blocks.len()).flat_map(|a| (0..blocks.len()).map(move |b| (a, b))).find(|&(a, b)| {
            let (x, y) = (&blocks[a], &blocks[b]);
            x.lo < y.lo && y.lo < x.hi && x.hi < y.hi
        });
        let Some((a, b)) = crossing else { break };
        if blocks[b].forward {
            blocks[b].lo = blocks[a].lo;
        } else if !blocks[a].forward {
            blocks[a].hi = blocks[b].hi;
        } else {
            let label = &stmts[blocks[b].lo];
            return Err(Diagnostic::new(
                label.span,
                format!(
                    "PascRust can't arrange these jumps as blocks: the `goto`s to `{}` jump back into the statements a `goto {}` jumps over{}",
                    blocks[b].name,
                    blocks[a].name,
                    super::rewrite_link("goto-blocks")
                ),
            ));
        }
    }
    // Outermost first: the earliest start, then the longest.
    blocks.sort_by_key(|b| (b.lo, std::cmp::Reverse(b.hi)));
    Ok(blocks)
}

/// The `goto` checks for one routine's body: every label used is declared
/// once and marks one statement, every `goto` reaches its label as a block
/// or loop, and no `Break` or `Continue` is caught inside one.
pub fn check(labels: &[Ident], body: &[Stmt], body_span: Span) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let declared: Vec<String> = labels.iter().map(Ident::key).collect();
    let mut marked: HashSet<String> = HashSet::new();
    // Each `goto`: its label, lowercase and as written, and where it is.
    let mut gotos: Vec<(String, String, Span)> = Vec::new();
    let mut each = |s: &Stmt| match &s.kind {
        StmtKind::Labeled { label, .. } => {
            if !declared.contains(&label.key()) {
                errors
                    .push(Diagnostic::new(label.span, format!("`{}` isn't declared in a `label` section", label.name)));
            } else if !marked.insert(label.key()) {
                errors.push(Diagnostic::new(
                    label.span,
                    format!("the label `{}` marks another statement already", label.name),
                ));
            }
        }
        StmtKind::Goto(label) => gotos.push((label.key(), label.name.clone(), label.span)),
        _ => {}
    };
    // Dead statements are comments in the Rust; their jumps don't count.
    let mut lists = Vec::new();
    all_lists(body, &mut lists);
    let mut dead_spans: Vec<Span> = Vec::new();
    for list in &lists {
        for (s, dead) in list.iter().zip(dead(list)) {
            if dead {
                dead_spans.push(s.span);
            }
        }
    }
    let live = |span: Span| !dead_spans.iter().any(|d| d.start <= span.start && span.end <= d.end);
    let whole = Stmt { kind: StmtKind::Compound(body.to_vec()), span: body_span };
    visit(&whole, &mut each);
    gotos.retain(|(_, _, span)| live(*span));
    for (key, name, span) in &gotos {
        if !marked.contains(key) {
            let message = if declared.contains(key) {
                format!("no statement is marked with the label `{name}`")
            } else {
                format!("`{name}` isn't declared in a `label` section")
            };
            errors.push(Diagnostic::new(*span, message));
        }
    }
    // Each statement list's blocks; a `goto` that no list's block reaches
    // jumps into a statement it isn't in.
    let mut reached: HashSet<Span> = HashSet::new();
    for list in lists {
        match plan(list) {
            Ok(blocks) => {
                for block in &blocks {
                    for s in &list[block.lo..block.hi] {
                        visit(s, &mut |s| {
                            if let StmtKind::Goto(label) = &s.kind
                                && label.key() == block.label
                            {
                                reached.insert(label.span);
                            }
                        });
                    }
                }
            }
            Err(e) => {
                errors.push(e);
                // The list's own `goto`s have had their error.
                let here: Vec<String> = list
                    .iter()
                    .filter_map(|s| match &s.kind {
                        StmtKind::Labeled { label, .. } => Some(label.key()),
                        _ => None,
                    })
                    .collect();
                for s in list {
                    visit(s, &mut |s| {
                        if let StmtKind::Goto(label) = &s.kind
                            && here.contains(&label.key())
                        {
                            reached.insert(label.span);
                        }
                    });
                }
            }
        }
    }
    for (key, name, span) in gotos {
        if marked.contains(&key) && !reached.contains(&span) {
            errors.push(Diagnostic::new(
                span,
                format!(
                    "this `goto {name}` jumps into a statement it isn't in; Rust can only leave a block or repeat a loop, so PascRust jumps only forward or back within a statement list the `goto` is in{}",
                    super::rewrite_link("goto-blocks")
                ),
            ));
        }
    }
    errors
}

/// `list` and every statement list nested in it. (The `'s` lifetime says
/// the lists borrow from the same statements as `list`.)
fn all_lists<'s>(list: &'s [Stmt], out: &mut Vec<&'s [Stmt]>) {
    out.push(list);
    for s in list {
        for inner in statement_lists(s) {
            all_lists(inner, out);
        }
    }
}

/// The labels on `s`: none, one, or a chain (`L1: L2: x`).
pub fn labels_of(s: &Stmt) -> Vec<&Ident> {
    match &s.kind {
        StmtKind::Labeled { label, body } => {
            let mut labels = vec![label];
            labels.extend(labels_of(body));
            labels
        }
        _ => Vec::new(),
    }
}

/// The statement lists directly in `s`. A single statement where a list
/// could be (`if c then L: x`) counts as a list of one.
pub fn statement_lists(s: &Stmt) -> Vec<&[Stmt]> {
    match &s.kind {
        StmtKind::Compound(body) | StmtKind::Repeat { body, .. } => vec![body],
        StmtKind::If { then_branch, else_branch, .. } => {
            let mut lists = vec![std::slice::from_ref(&**then_branch)];
            if let Some(e) = else_branch {
                lists.push(std::slice::from_ref(&**e));
            }
            lists
        }
        StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::With { body, .. } => {
            vec![std::slice::from_ref(&**body)]
        }
        // A label belongs to the list its statement is in; the lists are
        // those of the statement it labels.
        StmtKind::Labeled { body, .. } => statement_lists(body),
        StmtKind::Case { arms, else_body, .. } => {
            let mut lists: Vec<&[Stmt]> = arms.iter().map(|arm| std::slice::from_ref(&arm.body)).collect();
            if let Some(body) = else_body {
                lists.push(body);
            }
            lists
        }
        StmtKind::Empty | StmtKind::Assign { .. } | StmtKind::Call(_) | StmtKind::Goto(_) => Vec::new(),
    }
}

/// Whether a loop's `Break` and `Continue` are inside one of its goto
/// blocks (not in a loop nested in it): Rust's plain `break` can't leave a
/// labeled block (and in a goto loop it would leave that loop instead), so
/// then the loop needs a label of its own. `body` is the loop's body.
pub fn exits_in_blocks(body: &[Stmt]) -> (bool, bool) {
    // A loop's own `Break` and `Continue` stay inside it.
    fn exits(s: &Stmt, found: &mut (bool, bool)) {
        match &s.kind {
            StmtKind::While { .. } | StmtKind::For { .. } | StmtKind::Repeat { .. } => {}
            StmtKind::Call(e) => match &e.kind {
                ExprKind::Name(id) if id.key() == "break" => found.0 = true,
                ExprKind::Name(id) if id.key() == "continue" => found.1 = true,
                _ => {}
            },
            _ => {
                for list in statement_lists(s) {
                    list.iter().for_each(|s| exits(s, found));
                }
            }
        }
    }
    fn lists(list: &[Stmt], found: &mut (bool, bool)) {
        for block in plan(list).unwrap_or_default() {
            list[block.lo..block.hi].iter().for_each(|s| exits(s, found));
        }
        for s in list {
            if !matches!(s.kind, StmtKind::While { .. } | StmtKind::For { .. } | StmtKind::Repeat { .. }) {
                statement_lists(s).into_iter().for_each(|inner| lists(inner, found));
            }
        }
    }
    let mut found = (false, false);
    lists(body, &mut found);
    found
}
