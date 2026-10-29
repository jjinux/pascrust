//! Carrying the Pascal program's comments into the generated Rust.
//!
//! Codegen doesn't write things out in source order: constants move to
//! module level, a procedure's locals to the top of its function, nested
//! procedures out of their parents. So each comment is first attached to
//! an *anchor*, a construct whose Rust is written in one place: a
//! statement, a declaration, a record field, a procedure, or the `end` of
//! a block. A comment that ends the line an anchor ends on is that
//! anchor's *trailing* comment (`a: Integer; {pos of ch}`); any other
//! comment is a *leading* comment of the next anchor. Codegen then writes
//! each anchor's comments wherever it writes the anchor, as `//` lines.

use std::collections::HashMap;

use crate::ast::*;
use crate::diag::{SourceMap, Span};
use crate::lexer::Comment;

#[derive(Default)]
pub(super) struct Comments {
    /// Comment lines to write before an anchor, by its start offset.
    leading: HashMap<usize, Vec<String>>,
    /// A one-line comment to write at the end of an anchor's line.
    trailing: HashMap<usize, String>,
    /// The field groups of each record and object type, by the type's start
    /// offset: each group's field names (lowercase) and its anchor. Codegen
    /// writes record fields from sema's types, which have no positions.
    pub(super) field_groups: HashMap<usize, Vec<(Vec<String>, usize)>>,
    /// For `x, y: Integer; {position}`: each variable's name start, its
    /// group's anchor, and whether it's the group's last variable, whose
    /// `let` gets the group's comment.
    var_names: HashMap<usize, (usize, bool)>,
    /// Every type declaration: its anchor and the whole `Name = type` span.
    /// An alias (`TInt = Integer`) makes no Rust item, so codegen writes
    /// its comments on their own.
    pub(super) type_decls: Vec<(usize, Span)>,
}

impl Comments {
    pub(super) fn new(map: &SourceMap, program: &Program, units: &[Unit], comments: &[Comment]) -> Comments {
        let mut walker = Anchors {
            map,
            anchors: Vec::new(),
            field_groups: HashMap::new(),
            var_names: HashMap::new(),
            type_decls: Vec::new(),
            headings: Vec::new(),
            bodies: Vec::new(),
            headings_of_files: Vec::new(),
        };
        walker.heading(&program.name, program.uses.last().unwrap_or(&program.name).span.end);
        walker.block(&program.block);
        for unit in units {
            let end = unit.interface_uses.last().map_or(unit.interface_keyword, |u| u.span).end;
            walker.heading(&unit.name, end);
            walker.decls(&unit.interface);
            walker.decls(&unit.implementation);
            walker.stmts(&unit.init);
            walker.block_end(unit.end);
        }
        let mut anchors = walker.anchors;
        anchors.sort();
        anchors.dedup_by_key(|&mut (start, _)| start);
        let mut result = Comments {
            field_groups: walker.field_groups,
            var_names: walker.var_names,
            type_decls: walker.type_decls,
            ..Comments::default()
        };
        // Finding an offset's line by counting newlines from the start of
        // the file is slow when it's done for every comment and anchor, so
        // the newlines' offsets are listed once and searched.
        let lines = LineTable::new(map);
        // The anchors by the line they end on.
        let mut ending_on: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        for &(start, end) in &anchors {
            ending_on.entry(lines.line(end)).or_default().push((start, end));
        }
        for comment in comments {
            if comment.text.trim().is_empty() {
                continue; // `{}`
            }
            // The latest anchor that ends on the comment's line, before it.
            let before = ending_on
                .get(&lines.line(comment.span.start))
                .into_iter()
                .flatten()
                .filter(|&&(_, end)| end <= comment.span.start)
                .max_by_key(|&&(start, _)| start);
            if let Some(&(start, _)) = before {
                result.add_trailing(start, one_line(&comment.text));
                continue;
            }
            // (A comment ending the first line of a construct that goes on,
            // `for i := 1 to n do {each row}`, becomes a leading comment of
            // what's inside: rustfmt would move it after the `{` anyway.)
            // Otherwise, the next anchor in the same file. The anchors are
            // sorted, so `partition_point` finds it by binary search.
            let next = anchors.partition_point(|&(start, _)| start < comment.span.end);
            // But a comment inside a statement, with nothing after it there
            // (`b := 2 + {why} 3;`), is about that statement: the innermost
            // anchor around it. (Anchors nest, so it's the latest one that
            // starts before the comment and ends after it.)
            let around = anchors[..next].iter().rev().find(|&&(_, end)| end >= comment.span.end);
            if let Some(&(start, end)) = around
                && anchors.get(next).is_none_or(|&(next_start, _)| next_start >= end)
            {
                // Except that the file's heading keeps its comments whole.
                if walker.headings_of_files.contains(&start) {
                    result.leading.entry(start).or_default().extend(lines_of(&comment.text));
                } else {
                    result.add_trailing(start, one_line(&comment.text));
                }
                continue;
            }
            if let Some(&(start, _)) = anchors.get(next)
                && map.file_index(start) == map.file_index(comment.span.start)
            {
                result.leading.entry(start).or_default().extend(lines_of(&comment.text));
            }
        }
        // A heading (`forward`, in a unit's interface, or in an object type)
        // makes no Rust of its own: its comments go with its body, the
        // first one after it with the same name.
        for (key, heading) in walker.headings {
            let body = walker.bodies.iter().find(|(k, at)| *k == key && *at > heading);
            let Some(&(_, body)) = body else { continue };
            let mut lines = result.leading.remove(&heading).unwrap_or_default();
            lines.extend(result.trailing.remove(&heading));
            if !lines.is_empty() {
                let body_lines = result.leading.entry(body).or_default();
                lines.append(body_lines);
                *body_lines = lines;
            }
        }
        result
    }

    /// Add a comment to the end of the anchor's line, after any already there.
    fn add_trailing(&mut self, at: usize, text: String) {
        self.trailing
            .entry(at)
            .and_modify(|t| {
                t.push(' ');
                t.push_str(&text);
            })
            .or_insert(text);
    }

    /// Whether the anchor at `at` has comments not yet written.
    pub(super) fn has_any(&self, at: usize) -> bool {
        self.leading.contains_key(&at) || self.trailing.contains_key(&at)
    }

    /// The comment lines to write before the anchor at `at` (each once).
    pub(super) fn take_leading(&mut self, at: usize) -> Vec<String> {
        self.leading.remove(&at).unwrap_or_default()
    }

    /// The comment to write at the end of the anchor's line (once).
    pub(super) fn take_trailing(&mut self, at: usize) -> Option<String> {
        self.trailing.remove(&at)
    }

    /// The trailing comment for the variable (or typed constant) declared
    /// at `at`. In a group of variables, the last one gets it.
    pub(super) fn take_decl_trailing(&mut self, at: usize) -> Option<String> {
        match self.var_names.get(&at) {
            Some(&(anchor, true)) => self.trailing.remove(&anchor),
            Some(&(_, false)) => None,
            None => self.trailing.remove(&at),
        }
    }
}

/// Every line of every source file, numbered across the whole source map.
struct LineTable {
    /// Where each line starts, in order. A file's start also starts a line.
    starts: Vec<usize>,
}

impl LineTable {
    fn new(map: &SourceMap) -> LineTable {
        let mut starts = Vec::new();
        for i in 0..map.len() {
            let file = map.file(i);
            starts.push(file.base);
            let newlines = file.text.iter().enumerate().filter(|&(_, &b)| b == b'\n');
            starts.extend(newlines.map(|(at, _)| file.base + at + 1));
        }
        LineTable { starts }
    }

    /// The number of the line `offset` is on (unique across files).
    fn line(&self, offset: usize) -> usize {
        self.starts.partition_point(|&start| start <= offset)
    }
}

/// Finds the anchors in the AST, as `(start, end)` offsets.
struct Anchors<'a> {
    map: &'a SourceMap,
    anchors: Vec<(usize, usize)>,
    field_groups: HashMap<usize, Vec<(Vec<String>, usize)>>,
    var_names: HashMap<usize, (usize, bool)>,
    type_decls: Vec<(usize, Span)>,
    /// Procedure headings with no body, by name (`shape.area` for a
    /// method), and their anchors; and the procedures with bodies.
    headings: Vec<(String, usize)>,
    bodies: Vec<(String, usize)>,
    /// The anchors of `program Name; uses ...;` and `unit Name; ...`.
    headings_of_files: Vec<usize>,
}

impl Anchors<'_> {
    /// `program Name; uses A, B;` (or a unit's): comments before it or on
    /// its line describe the whole file.
    fn heading(&mut self, name: &Ident, end: usize) {
        self.anchors.push((name.span.start, end));
        self.headings_of_files.push(name.span.start);
    }

    fn block(&mut self, block: &Block) {
        self.decls(&block.decls);
        self.stmts(&block.body);
        self.block_end(block.body_span);
    }

    /// The `end` that closes a block, so comments just before it stay at
    /// the end of the block.
    fn block_end(&mut self, span: Span) {
        if let Some(at) = end_keyword(self.map, span) {
            self.anchors.push((at, at + 3));
        }
    }

    fn decls(&mut self, decls: &[Decl]) {
        for decl in decls {
            match decl {
                Decl::Const(c) => self.anchors.push((c.name.span.start, c.name.span.end)),
                Decl::Type(t) => {
                    // The type is the anchor: that's where `type_defs` puts it.
                    let span = t.ty.span();
                    self.anchors.push((span.start, span.end));
                    self.type_decls.push((span.start, t.name.span.to(span)));
                    self.type_expr(&t.ty, &t.name.key());
                }
                Decl::Var(v) => {
                    let at = v.names[0].span.start;
                    self.anchors.push((at, v.ty.span().end));
                    for (i, name) in v.names.iter().enumerate() {
                        self.var_names.insert(name.span.start, (at, i == v.names.len() - 1));
                    }
                    self.type_expr(&v.ty, "");
                }
                Decl::Proc(p) => {
                    self.anchors.push((p.span.start, p.header_span.end));
                    let key = match &p.owner {
                        Some(owner) => format!("{}.{}", owner.key(), p.name.key()),
                        None => p.name.key(),
                    };
                    if let ProcBody::Block(body) = &p.body {
                        self.bodies.push((key, p.span.start));
                        self.block(body);
                    } else {
                        self.headings.push((key, p.span.start));
                    }
                }
            }
        }
    }

    /// The fields of a record or object type, written out in a declaration.
    /// `name` is the declared type's name, for an object's methods.
    fn type_expr(&mut self, ty: &TypeExpr, name: &str) {
        let span = ty.span();
        let fields = match ty {
            TypeExpr::Record(fields, _) | TypeExpr::Object { fields, .. } => fields,
            TypeExpr::Array { elem, .. } => return self.type_expr(elem, ""),
            _ => return,
        };
        let mut groups = Vec::new();
        for group in fields {
            let at = group.names[0].span.start;
            self.anchors.push((at, group.ty.span().end));
            groups.push((group.names.iter().map(|n| n.key()).collect(), at));
            self.type_expr(&group.ty, "");
        }
        self.field_groups.insert(span.start, groups);
        if let TypeExpr::Object { methods, .. } = ty {
            for method in methods {
                self.anchors.push((method.span.start, method.header_span.end));
                self.headings.push((format!("{name}.{}", method.name.key()), method.span.start));
            }
        }
    }

    fn stmts(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        if s.kind == StmtKind::Empty {
            return;
        }
        self.anchors.push((s.span.start, s.span.end));
        match &s.kind {
            StmtKind::Compound(body) => {
                self.stmts(body);
                self.block_end(s.span);
            }
            StmtKind::If { then_branch, else_branch, .. } => {
                self.stmt(then_branch);
                if let Some(e) = else_branch {
                    self.stmt(e);
                }
            }
            StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::With { body, .. } => self.stmt(body),
            StmtKind::Repeat { body, cond } => {
                self.stmts(body);
                // The `until` test: comments before `until` stay in the loop.
                self.anchors.push((cond.span.start, cond.span.end));
            }
            StmtKind::Case { arms, else_body, .. } => {
                for arm in arms {
                    self.stmt(&arm.body);
                }
                if let Some(body) = else_body {
                    self.stmts(body);
                }
            }
            StmtKind::Empty | StmtKind::Assign { .. } | StmtKind::Call(_) => {}
        }
    }
}

/// Where the `end` keyword that ends `span` starts, if it does.
pub(super) fn end_keyword(map: &SourceMap, span: Span) -> Option<usize> {
    let at = span.end.checked_sub(3)?;
    map.slice(Span::new(at, span.end)).eq_ignore_ascii_case(b"end").then_some(at)
}

/// A comment's text as `//` lines: indentation shared by its lines
/// removed, and blank lines at the start and end dropped.
fn lines_of(text: &str) -> Vec<String> {
    let raw: Vec<&str> = text.lines().map(str::trim_end).collect();
    // The first line starts right after `{`, so its indentation doesn't
    // count; the others share the comment's indentation in the file.
    let indent = raw.iter().skip(1).filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min();
    let mut out: Vec<String> = raw
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let l = if i == 0 { l.trim_start() } else { l.get(indent.unwrap_or(0)..).unwrap_or(l.trim_start()) };
            if l.is_empty() { "//".to_string() } else { format!("// {l}") }
        })
        .collect();
    while out.first().is_some_and(|l| l == "//") {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l == "//") {
        out.pop();
    }
    out
}

/// A comment's text on one line, for the end of a line of code.
fn one_line(text: &str) -> String {
    format!("// {}", text.split_whitespace().collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_text_becomes_rust_lines() {
        assert_eq!(lines_of(" pos of ch "), ["// pos of ch"]);
        assert_eq!(
            lines_of("t\n   Shannon\n     indented\n\n   end\n"),
            ["// t", "// Shannon", "//   indented", "//", "// end"]
        );
        assert_eq!(one_line(" char for\n   encoding "), "// char for encoding");
    }
}
