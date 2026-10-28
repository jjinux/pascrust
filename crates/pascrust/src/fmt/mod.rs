//! `pascrust fmt`: make the indentation, spacing and identifier casing of
//! Turbo Pascal sources consistent, without changing what they mean.
//!
//! It's a re-indenter rather than a pretty-printer: line breaks stay where
//! the author put them, and strings and comments are never touched.
//!
//! - **Indentation** comes from the block structure, 3 spaces a level, in
//!   the style where everything belonging to a routine (its `var` section,
//!   nested routines and `begin ... end`) sits one level in from its heading:
//!
//!   ```pascal
//!   procedure Outer(x: integer);
//!      var
//!         y: integer;
//!      procedure Inner;
//!         begin
//!            writeln(x);
//!         end;
//!      begin
//!         if x > 0 then
//!            y := x
//!         else
//!            y := -x;
//!      end;
//!   ```
//!
//!   A continuation line (one that doesn't start a statement or declaration)
//!   moves as far as the line that started it, so hand alignment survives.
//! - **Spacing**: one space around `:=` and `=`, after `,` and `:`; none
//!   before `,`, `;` or a closing bracket, or after an opening one. Columns of
//!   declarations (`name     : string`) and of trailing comments stay lined
//!   up. `Write(x:8:2)` formats are left alone.
//! - **Casing**: keywords are lowercase, and each identifier is spelled the
//!   way it's spelled most often across all the files formatted together.
//!   Pascal is case-insensitive, so this can't change a program either.
//!
//! The output has UNIX line endings; DOS (`\r\n`) input is fine.
//!
//! The work happens in three passes: [`lines`] splits the source into lines
//! of items (code tokens, spaces, comments), [`structure`] finds the
//! indentation level of every token that can start a line, and [`layout`]
//! re-indents and re-spaces each line.

mod layout;
mod lines;
mod structure;

use std::collections::HashMap;

use crate::diag::Diagnostic;
use crate::lexer::tokenize;
use crate::token::{Token, TokenKind};

/// Spaces per indentation level.
const INDENT: usize = 3;

/// Words that aren't reserved in Turbo Pascal but act like keywords where
/// they appear. They're written in lowercase, like keywords.
const DIRECTIVES: &[&str] = &[
    "absolute",
    "assembler",
    "external",
    "far",
    "forward",
    "interrupt",
    "near",
    "otherwise",
    "private",
    "public",
    "virtual",
];

/// The canonical spelling of every identifier in a set of files.
pub struct Casing {
    /// Lowercase name to spelling.
    spellings: HashMap<String, String>,
}

impl Casing {
    /// Count the spellings of each identifier in `sources` and pick the
    /// most frequent one (on a tie, the one seen first).
    pub fn new(sources: &[&[u8]]) -> Result<Casing, (usize, Diagnostic)> {
        // For each lowercase name: spelling -> (count, order first seen).
        let mut counts: HashMap<String, HashMap<String, (usize, usize)>> = HashMap::new();
        let mut seen = 0;
        for (index, source) in sources.iter().enumerate() {
            let source = normalize_newlines(source);
            for token in tokenize(&source).map_err(|d| (index, d))? {
                if let TokenKind::Ident(name) = token.kind {
                    let entry = counts.entry(name.to_ascii_lowercase()).or_default();
                    // `or_insert_with` runs its closure only for a new key,
                    // so `seen` counts distinct spellings in order.
                    let (count, _) = entry.entry(name).or_insert_with(|| {
                        seen += 1;
                        (0, seen)
                    });
                    *count += 1;
                }
            }
        }
        let spellings = counts
            .into_iter()
            .map(|(key, spellings)| {
                // `Reverse` flips the ordering, so `max_by_key` prefers the
                // spelling seen *first* among equally frequent ones.
                let best = spellings
                    .into_iter()
                    .max_by_key(|(_, (count, order))| (*count, std::cmp::Reverse(*order)))
                    .map(|(spelling, _)| spelling)
                    .unwrap_or_default();
                (key, best)
            })
            .collect();
        Ok(Casing { spellings })
    }

    /// How to write `token`, which is at `text` in the source.
    fn spell(&self, token: &Token, text: &[u8]) -> Vec<u8> {
        match &token.kind {
            TokenKind::Keyword(k) => k.as_str().as_bytes().to_vec(),
            TokenKind::Ident(name) => {
                let key = name.to_ascii_lowercase();
                if DIRECTIVES.contains(&key.as_str()) {
                    key.into_bytes()
                } else {
                    self.spellings.get(&key).map_or_else(|| text.to_vec(), |s| s.as_bytes().to_vec())
                }
            }
            _ => text.to_vec(),
        }
    }
}

/// Format one source file, using identifier spellings from `casing`.
pub fn format_source(source: &[u8], casing: &Casing) -> Result<Vec<u8>, Diagnostic> {
    let source = normalize_newlines(source);
    let tokens = tokenize(&source)?;
    let lines = lines::split(&source, &tokens, casing);
    let code: Vec<&Token> = tokens.iter().filter(|t| is_code(t)).collect();
    let levels = structure::levels(&code, &lines);
    Ok(layout::layout(&lines, &levels))
}

/// Format several files that belong together (a program and its units), so
/// that identifiers are spelled the same in all of them.
pub fn format_files(sources: &[&[u8]]) -> Result<Vec<Vec<u8>>, (usize, Diagnostic)> {
    let casing = Casing::new(sources)?;
    sources.iter().enumerate().map(|(index, source)| format_source(source, &casing).map_err(|d| (index, d))).collect()
}

/// Tokens that are code, as opposed to directives (comments to us) and the
/// end-of-file marker.
fn is_code(token: &Token) -> bool {
    !matches!(token.kind, TokenKind::Directive(_) | TokenKind::Eof)
}

/// DOS `\r\n` (and old Mac `\r`) line endings become `\n`.
fn normalize_newlines(source: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(source.len());
    let mut bytes = source.iter().peekable();
    while let Some(&b) = bytes.next() {
        if b == b'\r' {
            if bytes.peek() == Some(&&b'\n') {
                continue; // the `\n` comes next
            }
            out.push(b'\n');
        } else {
            out.push(b);
        }
    }
    out
}

#[cfg(test)]
mod tests;
