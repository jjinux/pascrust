//! Splitting a source file into lines of items.
//!
//! The lexer throws comments away, but every token knows its byte span, so
//! the text *between* two tokens is exactly the whitespace and comments
//! there. We scan those gaps here, and the lexer and parser stay simple.

use super::Casing;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Space,
    Code,
    Comment,
    /// The rest of a comment that started on an earlier line.
    CommentCont,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: Kind,
    /// The text as it will be written (identifiers already re-cased).
    pub text: Vec<u8>,
    /// For code, its index among the code tokens.
    pub tok: Option<usize>,
    /// A comment that continues on the next line.
    pub multi: bool,
    /// The column it started at in the original source.
    pub old_col: usize,
}

impl Item {
    fn new(kind: Kind, text: &[u8]) -> Item {
        Item { kind, text: text.to_vec(), tok: None, multi: false, old_col: 0 }
    }

    pub fn is(&self, text: &str) -> bool {
        self.kind == Kind::Code && self.text.eq_ignore_ascii_case(text.as_bytes())
    }

    /// Is this an identifier or keyword (as opposed to a symbol or literal)?
    pub fn is_word(&self) -> bool {
        self.kind == Kind::Code && self.text.first().is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
    }
}

#[derive(Debug, Default)]
pub struct Line {
    /// The width of the original indentation.
    pub old_indent: usize,
    /// Everything after the indentation.
    pub items: Vec<Item>,
}

impl Line {
    pub fn first(&self) -> Option<&Item> {
        self.items.first()
    }
}

/// The display width of a run of spaces and tabs (tab stops every 8).
pub fn width(text: &[u8]) -> usize {
    text.iter().fold(0, |w, &c| if c == b'\t' { w + 8 - w % 8 } else { w + 1 })
}

pub fn split(source: &[u8], tokens: &[Token], casing: &Casing) -> Vec<Line> {
    let mut raw: Vec<Vec<Item>> = vec![Vec::new()];
    let mut pos = 0;
    let mut code_index = 0;
    for token in tokens {
        if token.kind == TokenKind::Eof {
            break;
        }
        gap(&source[pos..token.span.start], &mut raw);
        let text = &source[token.span.start..token.span.end];
        if let TokenKind::Directive(_) = token.kind {
            comment(text, &mut raw);
        } else {
            let mut item = Item::new(Kind::Code, &casing.spell(token, text));
            item.tok = Some(code_index);
            code_index += 1;
            // `last_mut().unwrap()` can't fail: `raw` never becomes empty.
            raw.last_mut().unwrap().push(item);
        }
        pos = token.span.end;
    }
    gap(&source[pos..], &mut raw);

    raw.into_iter()
        .map(|mut items| {
            let old_indent = match items.first() {
                Some(item) if item.kind == Kind::Space => width(&items.remove(0).text),
                _ => 0,
            };
            let mut col = old_indent;
            for item in &mut items {
                item.old_col = col;
                col += if item.kind == Kind::Space { width(&item.text) } else { item.text.len() };
            }
            Line { old_indent, items }
        })
        .collect()
}

/// Scan the whitespace and comments between two tokens.
fn gap(text: &[u8], raw: &mut Vec<Vec<Item>>) {
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let len = match rest[0] {
            b'\n' => {
                raw.push(Vec::new());
                1
            }
            b' ' | b'\t' => {
                let n = rest.iter().take_while(|c| matches!(c, b' ' | b'\t')).count();
                raw.last_mut().unwrap().push(Item::new(Kind::Space, &rest[..n]));
                n
            }
            b'{' => comment_len(rest, 1, b"}"),
            b'(' if rest.starts_with(b"(*") => comment_len(rest, 2, b"*)"),
            b'/' if rest.starts_with(b"//") => rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len()),
            // Anything else the lexer skipped, i.e. a DOS Ctrl-Z end-of-file
            // marker, is dropped.
            _ => {
                i += 1;
                continue;
            }
        };
        if matches!(rest[0], b'{' | b'(' | b'/') {
            comment(&rest[..len], raw);
        }
        i += len;
    }
}

/// The length of a comment that starts `text` with an `open`-byte opener
/// and ends with `close`.
fn comment_len(text: &[u8], open: usize, close: &[u8]) -> usize {
    text[open..].windows(close.len()).position(|w| w == close).map_or(text.len(), |p| open + p + close.len())
}

/// Add a comment, splitting it if it spans lines.
fn comment(text: &[u8], raw: &mut Vec<Vec<Item>>) {
    let mut parts = text.split(|&c| c == b'\n');
    let first = parts.next().unwrap_or_default();
    let mut item = Item::new(Kind::Comment, first);
    item.multi = text.contains(&b'\n');
    raw.last_mut().unwrap().push(item);
    for part in parts {
        let indent = part.iter().take_while(|c| matches!(c, b' ' | b'\t')).count();
        let mut line = Vec::new();
        if indent > 0 {
            line.push(Item::new(Kind::Space, &part[..indent]));
        }
        line.push(Item::new(Kind::CommentCont, &part[indent..]));
        raw.push(line);
    }
}
