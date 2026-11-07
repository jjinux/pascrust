//! Re-indenting and re-spacing lines, given the levels from `structure`.

use std::collections::HashMap;

use super::INDENT;
use super::lines::{Item, Kind, Line};

/// Calls whose `:` arguments are formats (`Write(x:8:2)`), not declarations.
const WRITE_LIKE: &[&str] = &["write", "writeln", "str"];

pub fn layout(lines: &[Line], levels: &[Option<usize>]) -> Vec<u8> {
    let indents = indentation(lines, levels);
    let groups = colon_groups(lines, &indents);

    let mut out: Vec<Vec<u8>> = Vec::new();
    // Old trailing-comment columns on the previous line -> new ones.
    let mut columns = HashMap::new();
    // How far the first line of the current multi-line comment moved.
    let mut comment_shift = 0isize;
    for (n, line) in lines.iter().enumerate() {
        let Some(first) = line.first() else {
            out.push(Vec::new());
            continue;
        };
        if first.kind == Kind::CommentCont {
            // Inside a comment: keep its shape, moved with its first line.
            let text: Vec<u8> = line.items.iter().flat_map(|i| i.text.iter().copied()).collect();
            let indent = (line.old_indent as isize + comment_shift).max(0) as usize;
            out.push(if text.iter().all(u8::is_ascii_whitespace) { Vec::new() } else { indented(indent, &text) });
            continue;
        }
        let indent = indents[n];
        if line.items.iter().any(|i| i.multi) {
            comment_shift = indent as isize - line.old_indent as isize;
        }
        let target = groups.get(&n).map(|group| {
            // Line the group's colons up one space after the longest name:
            // measure where each line's colon lands without padding first.
            1 + group
                .iter()
                .map(|&m| respace(&lines[m], None, indents[m], &mut columns.clone()).1.unwrap_or(0))
                .max()
                .unwrap_or(0)
        });
        let (text, _) = respace(line, target, indent, &mut columns);
        out.push(indented(indent, &text));
    }

    // At most one blank line in a row, none at the ends, and a final newline.
    let mut text = Vec::new();
    let mut blank = false;
    for line in out {
        if line.is_empty() {
            blank = !text.is_empty();
            continue;
        }
        if blank {
            text.push(b'\n');
            blank = false;
        }
        text.extend(line);
        text.push(b'\n');
    }
    text
}

fn indented(indent: usize, text: &[u8]) -> Vec<u8> {
    let mut line = vec![b' '; indent];
    line.extend_from_slice(text);
    line.truncate(line.iter().rposition(|c| !c.is_ascii_whitespace()).map_or(0, |p| p + 1));
    line
}

/// The new indentation of every line (0 for blank lines and the inside of
/// comments, which are handled separately).
fn indentation(lines: &[Line], levels: &[Option<usize>]) -> Vec<usize> {
    let mut new: Vec<Option<usize>> = vec![None; lines.len()];
    // How far the last line that started a statement or declaration moved,
    // and where it ended up: continuation lines move with it.
    let mut shift = 0isize;
    let mut anchor = 0;
    for (n, line) in lines.iter().enumerate() {
        let Some(first) = line.first() else { continue };
        if first.kind != Kind::Code {
            continue; // comments are placed below
        }
        let indent = match first.tok.and_then(|t| levels[t]) {
            Some(level) => {
                let indent = level * INDENT;
                shift = indent as isize - line.old_indent as isize;
                anchor = indent;
                indent
            }
            None => ((line.old_indent as isize + shift).max(0) as usize).max(anchor + INDENT),
        };
        new[n] = Some(indent);
    }

    // A comment line goes with the code after it; before a closing `end` or
    // `until`, it lines up with the statements being closed.
    let mut following = None;
    for (n, line) in lines.iter().enumerate().rev() {
        let Some(first) = line.first() else { continue };
        match new[n] {
            Some(indent) => {
                following = Some(indent + if first.is("end") || first.is("until") { INDENT } else { 0 });
            }
            None if first.kind == Kind::Comment => new[n] = Some(following.unwrap_or(line.old_indent)),
            None => {}
        }
    }
    new.into_iter().map(|n| n.unwrap_or(0)).collect()
}

/// The old column of the first `:` outside brackets, and whether there was
/// a space before it.
fn colon(line: &Line) -> Option<(usize, bool)> {
    let mut depth = 0i32;
    for (k, item) in line.items.iter().enumerate() {
        if item.is("(") || item.is("[") {
            depth += 1;
        } else if item.is(")") || item.is("]") {
            depth -= 1;
        } else if item.is(":") && depth == 0 {
            let spaced = k > 0 && line.items[k - 1].kind == Kind::Space;
            return Some((item.old_col, spaced));
        }
    }
    None
}

/// Runs of adjacent lines at one indentation that each have a colon
/// (declarations, case arms) whose colons were lined up: two of them were in
/// the same column, and at least one had space before it, as in
/// `name     : string`. Maps each line to its group.
///
/// Both conditions survive formatting, which is what makes formatting
/// twice give the same result as formatting once.
fn colon_groups(lines: &[Line], indents: &[usize]) -> HashMap<usize, Vec<usize>> {
    let colons: Vec<_> = lines
        .iter()
        .map(|line| match line.first() {
            Some(first) if first.kind == Kind::Code => colon(line),
            _ => None,
        })
        .collect();
    let mut groups = HashMap::new();
    let mut n = 0;
    while n < lines.len() {
        let mut m = n;
        while colons[n].is_some() && m + 1 < lines.len() && colons[m + 1].is_some() && indents[m + 1] == indents[n] {
            m += 1;
        }
        let run: Vec<usize> = (n..=m).collect();
        let cols: Vec<(usize, bool)> = run.iter().filter_map(|&k| colons[k]).collect();
        let shared = cols.iter().enumerate().any(|(i, a)| cols[i + 1..].iter().any(|b| a.0 == b.0));
        if run.len() > 1 && shared && cols.iter().any(|c| c.1) {
            for &k in &run {
                groups.insert(k, run.clone());
            }
        }
        n = m + 1;
    }
    groups
}

/// Normalize the spaces between the code on one line (the indentation is
/// added by the caller).
///
/// - `colon_target`: the column (after the indentation) for the line's first
///   `:` outside brackets, to line up a group of declarations.
/// - `columns`: the old and new columns of the previous line's trailing
///   comments. A trailing comment keeps its old column, moved as far as its
///   line moved, unless the line before had a comment in the same old column;
///   then it joins that one, so lined-up comments stay lined up.
///
/// Returns the text and the column the colon has without padding.
fn respace(
    line: &Line,
    colon_target: Option<usize>,
    indent: usize,
    columns: &mut HashMap<usize, usize>,
) -> (Vec<u8>, Option<usize>) {
    let items = &line.items;
    // `Cow`-like: each output piece borrows from an item or is a fresh space.
    let mut out: Vec<(Kind, Vec<u8>)> = Vec::new();
    let mut colon_pos = None;
    // For each open bracket, the word before it (to spot `Write(`).
    let mut parens: Vec<Vec<u8>> = Vec::new();
    let mut prev_word: Vec<u8> = Vec::new();
    let space = |n: usize| (Kind::Space, vec![b' '; n]);
    let len = |out: &[(Kind, Vec<u8>)]| out.iter().map(|(_, t)| t.len()).sum::<usize>();

    let mut i = 0;
    while i < items.len() {
        let item = &items[i];
        let next = items.get(i + 1);
        let next_is_space = next.is_some_and(|n| n.kind == Kind::Space);
        // The item after `item`, skipping a space.
        let after = if next_is_space { items.get(i + 2) } else { next };
        let spaced_after = after.is_some_and(|a| a.kind != Kind::Comment);
        let in_write = parens.last().is_some_and(|w| WRITE_LIKE.iter().any(|x| w.eq_ignore_ascii_case(x.as_bytes())));

        if item.kind == Kind::Space {
            let keep = match next {
                None => false, // trailing whitespace
                Some(next) => {
                    !matches!(out.last(), Some((_, t)) if t == b"(" || t == b"[")
                        && !(next.is(")") || next.is("]") || next.is(",") || next.is(";"))
                }
            };
            if keep {
                out.push((Kind::Space, item.text.clone()));
            }
            i += 1;
            continue;
        }
        if item.is("(") || item.is("[") {
            parens.push(prev_word.clone());
        } else if item.is(")") || item.is("]") {
            parens.pop();
        }

        let step = if next_is_space { 2 } else { 1 };
        if [":=", "=", "<>", "<=", ">="].iter().any(|op| item.is(op)) {
            // One space each side, but more than one before `=` is
            // alignment (`outta5  = 0..5;`).
            let aligned = item.is("=") && matches!(out.last(), Some((Kind::Space, t)) if t.len() > 1);
            if !aligned {
                trim_space(&mut out);
                if !out.is_empty() {
                    out.push(space(1));
                }
            }
            out.push((Kind::Code, item.text.clone()));
            if spaced_after {
                out.push(space(1));
            }
            i += step;
            continue;
        }
        if item.is(":") && !in_write {
            trim_space(&mut out);
            if colon_pos.is_none() && parens.is_empty() {
                let pos = len(&out);
                colon_pos = Some(pos);
                if let Some(target) = colon_target {
                    out.push(space(target.saturating_sub(pos).max(1)));
                }
            }
            out.push((Kind::Code, item.text.clone()));
            if spaced_after {
                out.push(space(1));
            }
            i += step;
            continue;
        }
        if item.is(",") {
            trim_space(&mut out);
            out.push((Kind::Code, item.text.clone()));
            if spaced_after {
                out.push(space(1));
                i += step;
            } else {
                i += 1;
            }
            continue;
        }

        out.push((item.kind, item.text.clone()));
        if item.is_word() {
            prev_word = item.text.clone();
        } else if item.kind != Kind::Comment {
            prev_word.clear();
        }
        i += 1;
    }

    // Put trailing comments back in their columns.
    let mut comments = items.iter().filter(|i| i.kind == Kind::Comment);
    let mut new_columns = HashMap::new();
    let mut text: Vec<u8> = Vec::new();
    for (kind, part) in out {
        if kind == Kind::Comment {
            // There's one output comment per input comment, in order.
            let comment: &Item = comments.next().expect("comments are copied in order");
            trim_end(&mut text);
            if !text.is_empty() {
                let moved = (comment.old_col as isize + indent as isize - line.old_indent as isize).max(0) as usize;
                let target = columns.get(&comment.old_col).copied().unwrap_or(moved);
                let pad = target.saturating_sub(indent + text.len()).max(1);
                text.extend(std::iter::repeat_n(b' ', pad));
                new_columns.insert(comment.old_col, indent + text.len());
            }
        }
        text.extend(part);
    }
    *columns = new_columns;
    trim_end(&mut text);
    (text, colon_pos)
}

fn trim_space(out: &mut Vec<(Kind, Vec<u8>)>) {
    if matches!(out.last(), Some((Kind::Space, _))) {
        out.pop();
    }
}

fn trim_end(text: &mut Vec<u8>) {
    while text.last().is_some_and(u8::is_ascii_whitespace) {
        text.pop();
    }
}
