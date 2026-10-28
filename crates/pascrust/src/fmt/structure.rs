//! Finding the indentation level of each token that can start a line.
//!
//! This is a rough recursive-descent pass over the tokens: it knows the
//! shape of declarations and statements (where blocks open and close, which
//! statement a `then` or `do` governs) but skips over expressions and types
//! without looking inside. It never fails; on code it doesn't understand it
//! just stops marking levels, and those lines keep their relative
//! indentation.

use super::DIRECTIVES;
use super::lines::Line;
use crate::token::{Token, TokenKind};

const SECTIONS: &[&str] = &["const", "type", "var", "label"];
const ROUTINES: &[&str] = &["procedure", "function", "constructor", "destructor"];

/// The level of each code token that gets one, by code-token index.
pub fn levels(code: &[&Token], lines: &[Line]) -> Vec<Option<usize>> {
    let mut line_of = vec![0; code.len()];
    for (n, line) in lines.iter().enumerate() {
        for item in &line.items {
            if let Some(tok) = item.tok {
                line_of[tok] = n;
            }
        }
    }
    let words = code.iter().map(|t| word(t)).collect();
    let mut s = Structure { words, line_of, pos: 0, levels: vec![None; code.len()] };
    s.file();
    s.levels
}

/// What the structure pass matches tokens by: keywords, identifiers in
/// lowercase and punctuation. Literals are empty, so a string `'end'` never
/// looks like the keyword.
fn word(token: &Token) -> String {
    match &token.kind {
        TokenKind::Keyword(k) => k.as_str().to_string(),
        TokenKind::Ident(name) => name.to_ascii_lowercase(),
        TokenKind::Integer(_) | TokenKind::Real(_) | TokenKind::Str(_) | TokenKind::Directive(_) | TokenKind::Eof => {
            String::new()
        }
        punct => punct.punct_str().to_string(),
    }
}

struct Structure {
    words: Vec<String>,
    line_of: Vec<usize>,
    pos: usize,
    levels: Vec<Option<usize>>,
}

impl Structure {
    fn peek(&self) -> &str {
        self.at(0)
    }

    /// The word `offset` tokens from here (negative looks back), or "".
    fn at(&self, offset: isize) -> &str {
        self.pos.checked_add_signed(offset).and_then(|i| self.words.get(i)).map_or("", String::as_str)
    }

    fn done(&self) -> bool {
        self.pos >= self.words.len()
    }

    fn is_any(&self, words: &[&str]) -> bool {
        words.contains(&self.peek())
    }

    /// An identifier that isn't a directive, i.e. something being declared.
    fn at_name(&self) -> bool {
        let w = self.peek();
        w.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && crate::token::Keyword::from_lowercase(w).is_none()
            && !DIRECTIVES.contains(&w)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect(&mut self, word: &str) {
        if self.peek() == word {
            self.advance();
        }
    }

    /// Give the current token `level`, unless it already has one.
    fn mark(&mut self, level: usize) {
        if let Some(slot) = self.levels.get_mut(self.pos) {
            slot.get_or_insert(level);
        }
    }

    /// Skip to (not past) the next token in `stops` outside brackets.
    fn skip_until(&mut self, stops: &[&str]) {
        let mut depth = 0usize;
        while !self.done() {
            match self.peek() {
                w if depth == 0 && stops.contains(&w) => return,
                "(" | "[" => depth += 1,
                ")" | "]" => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.advance();
        }
    }

    // Files and blocks.

    fn file(&mut self) {
        if self.peek() == "unit" {
            self.mark(0);
            self.skip_until(&[";"]);
            self.expect(";");
            self.unit();
            return;
        }
        if self.peek() == "program" {
            self.mark(0);
            self.skip_until(&[";"]);
            self.expect(";");
        }
        self.block(0);
        self.expect(".");
    }

    /// A unit's parts sit at level 0 and their contents at level 1.
    fn unit(&mut self) {
        if self.peek() == "interface" {
            self.mark(0);
            self.advance();
            self.decls(1, false);
        }
        if self.peek() == "implementation" {
            self.mark(0);
            self.advance();
            self.decls(1, true);
        }
        if self.peek() == "begin" {
            self.mark(0);
            self.advance();
            self.stmt_list(1);
        }
        if self.peek() == "end" {
            self.mark(0);
            self.advance();
        }
        self.expect(".");
    }

    /// Declarations, then `begin ... end`, all at `level`.
    fn block(&mut self, level: usize) {
        self.decls(level, true);
        if self.peek() == "begin" {
            self.mark(level);
            self.advance();
            self.stmt_list(level + 1);
            self.mark(level);
            self.expect("end");
        }
    }

    /// `uses`, `const`/`type`/`var`/`label` sections and routines. In a
    /// unit's interface, routine headings have no `bodies`.
    fn decls(&mut self, level: usize, bodies: bool) {
        while !self.done() {
            if self.peek() == "uses" {
                self.mark(level);
                self.skip_until(&[";"]);
                self.expect(";");
            } else if self.is_any(SECTIONS) {
                self.mark(level);
                self.advance();
                while self.at_name() {
                    self.mark(level + 1);
                    self.declaration(level + 1);
                }
            } else if self.is_any(ROUTINES) {
                self.routine(level, bodies);
            } else {
                return;
            }
        }
    }

    /// One `name = ...;` or `name: ...;`, looking inside records and
    /// objects.
    fn declaration(&mut self, level: usize) {
        let mut depth = 0usize;
        while !self.done() {
            match self.peek() {
                ";" if depth == 0 => {
                    self.advance();
                    return;
                }
                "record" | "object" if self.at(-1) != "of" => {
                    self.advance();
                    self.fields(level);
                    continue;
                }
                "(" | "[" => depth += 1,
                ")" | "]" => depth = depth.saturating_sub(1),
                // A missing `;`: don't swallow the next section. (A
                // procedure *type*, `T = procedure(x: integer);`, follows
                // `=` or `:`.)
                w if depth == 0
                    && (SECTIONS.contains(&w) || ROUTINES.contains(&w) && !matches!(self.at(-1), "=" | ":")) =>
                {
                    return;
                }
                _ => {}
            }
            self.advance();
        }
    }

    /// The inside of `record ... end` or `object ... end`: fields and method
    /// headings one level in, `end` back at the declaration's level.
    fn fields(&mut self, level: usize) {
        while !self.done() && self.peek() != "end" {
            if self.peek() == ";" {
                self.advance();
            } else if self.is_any(&["private", "public"]) {
                self.mark(level);
                self.advance();
            } else if self.is_any(ROUTINES) {
                self.mark(level + 1);
                self.routine_heading();
            } else if self.peek() == "case" {
                // A variant part: `case tag: T of 1: (fields); ...`.
                self.mark(level + 1);
                self.skip_until(&["of"]);
                self.advance();
                while !self.done() && self.peek() != "end" {
                    self.mark(level + 2);
                    self.skip_until(&[";", "end"]);
                    self.expect(";");
                }
            } else {
                self.mark(level + 1);
                self.skip_until(&[";", "end"]);
            }
        }
        self.mark(level);
        self.expect("end");
    }

    /// `procedure Name(params): T;` and any directives after it. Returns
    /// false if no body follows (`forward`, `external`).
    fn routine_heading(&mut self) -> bool {
        self.advance();
        self.skip_until(&[";"]);
        self.expect(";");
        let mut body = true;
        while self.is_any(&["forward", "external", "far", "near", "interrupt", "assembler", "virtual", "inline"]) {
            if self.is_any(&["forward", "external", "inline"]) {
                body = false;
            }
            self.skip_until(&[";"]);
            self.expect(";");
        }
        body
    }

    fn routine(&mut self, level: usize, bodies: bool) {
        self.mark(level);
        if self.routine_heading() && bodies {
            self.block(level + 1);
            self.expect(";");
        }
    }

    // Statements.

    fn stmt_list(&mut self, level: usize) {
        while !self.done() {
            match self.peek() {
                "end" | "until" => return,
                ";" => self.advance(),
                _ => {
                    let before = self.pos;
                    self.stmt(level);
                    if self.pos == before {
                        self.advance(); // a stray token, such as `else` without `if`
                    }
                }
            }
        }
    }

    /// The statement after `then`, `else`, `do` or a case label (the token
    /// just before the current one). It goes one level in if it starts a new
    /// line, unless it's a `begin`, which lines up with its `if` or `while`.
    fn child(&mut self, level: usize) {
        if self.done() || self.is_any(&[";", "end", "else", "until"]) {
            return; // an empty statement
        }
        let same_line = self.line_of[self.pos] == self.line_of[self.pos - 1];
        if same_line || self.peek() == "begin" {
            self.stmt(level);
        } else {
            self.stmt(level + 1);
        }
    }

    fn stmt(&mut self, level: usize) {
        self.mark(level);
        match self.peek() {
            "begin" => {
                self.advance();
                self.stmt_list(level + 1);
                self.mark(level);
                self.expect("end");
            }
            "if" => {
                self.skip_until(&["then"]);
                self.advance();
                self.child(level);
                if self.peek() == "else" {
                    self.mark(level);
                    self.advance();
                    self.child(level);
                }
            }
            "while" | "for" | "with" => {
                self.skip_until(&["do"]);
                self.advance();
                self.child(level);
            }
            "repeat" => {
                self.advance();
                self.stmt_list(level + 1);
                self.mark(level);
                self.expect("until");
                self.skip_until(&[";", "end", "else", "until"]);
            }
            "case" => {
                self.skip_until(&["of"]);
                self.advance();
                while !self.done() && !self.is_any(&["end", "else", "otherwise"]) {
                    if self.peek() == ";" {
                        self.advance();
                        continue;
                    }
                    self.mark(level + 1);
                    self.skip_until(&[":"]);
                    self.advance();
                    self.child(level + 1);
                }
                if self.is_any(&["else", "otherwise"]) {
                    self.mark(level + 1);
                    self.advance();
                    self.stmt_list(level + 2);
                }
                self.mark(level);
                self.expect("end");
            }
            _ => self.skip_until(&[";", "end", "else", "until"]),
        }
    }
}
