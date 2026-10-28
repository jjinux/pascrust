//! The lexer turns Pascal source bytes into [`Token`]s.
//!
//! It works on bytes (`&[u8]`) rather than `&str` because Turbo Pascal
//! sources are DOS text files: they may contain code page 437 bytes that
//! aren't valid UTF-8, and they use `\r\n` line endings.
//!
//! Interesting Pascal lexing rules handled here:
//! - Keywords and identifiers are case-insensitive (`BEGIN`, `Begin`, `begin`).
//! - Three comment styles: `{ ... }`, `(* ... *)` and (Turbo Pascal 7) `// ...`.
//!   A comment starting with `$` is a compiler directive, such as `{$I-}`.
//! - `1..5` is three tokens (1, `..`, 5) but `1.5` is one real number, so the
//!   lexer must look two characters ahead after a digit.
//! - Strings use single quotes; a quote inside a string is doubled (`'it''s'`).
//!   Control characters can be spliced in with `#13` or `^M`, and adjacent
//!   pieces form one literal: `'line'#13#10`.
//! - `$FF` is a hexadecimal integer.

use crate::diag::{Diagnostic, Span};
use crate::token::{Keyword, Token, TokenKind};

/// Tokenize a whole source file. The last token is always [`TokenKind::Eof`].
pub fn tokenize(source: &[u8]) -> Result<Vec<Token>, Diagnostic> {
    let mut lexer = Lexer {
        src: source,
        pos: 0,
        tokens: Vec::new(),
    };
    lexer.run()?;
    Ok(lexer.tokens)
}

struct Lexer<'a> {
    // `'a` is a lifetime parameter: the lexer borrows the source bytes and
    // must not outlive them. The compiler checks this for us.
    src: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    /// The byte `offset` positions ahead of the cursor, or 0 at end of input.
    /// Using 0 as an "end" sentinel keeps the matching code simple; a NUL byte
    /// never appears in a valid Pascal source.
    fn peek_at(&self, offset: usize) -> u8 {
        self.src.get(self.pos + offset).copied().unwrap_or(0)
    }

    fn peek(&self) -> u8 {
        self.peek_at(0)
    }

    fn error(&self, start: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(Span::new(start, self.pos.max(start + 1)), message)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn run(&mut self) -> Result<(), Diagnostic> {
        loop {
            self.skip_whitespace_and_comments()?;
            let start = self.pos;
            let c = self.peek();
            if c == 0 && self.pos >= self.src.len() {
                self.push(TokenKind::Eof, start);
                return Ok(());
            }
            match c {
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.identifier(),
                b'0'..=b'9' => self.number()?,
                b'$' => self.hex_number()?,
                b'\'' | b'#' => self.string()?,
                b'^' if self.caret_is_control_char() => self.string()?,
                _ => self.punctuation()?,
            }
            // Directives are emitted by `skip_whitespace_and_comments`.
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), Diagnostic> {
        loop {
            let start = self.pos;
            match (self.peek(), self.peek_at(1)) {
                (b' ' | b'\t' | b'\r' | b'\n' | 0x1a, _) if self.pos < self.src.len() => {
                    // 0x1a is Ctrl-Z, the DOS end-of-file marker some old
                    // editors appended.
                    self.pos += 1;
                }
                (b'{', _) => {
                    self.pos += 1;
                    let body_start = self.pos;
                    while self.peek() != b'}' {
                        if self.pos >= self.src.len() {
                            return Err(self.error(start, "unterminated `{` comment"));
                        }
                        self.pos += 1;
                    }
                    let body_end = self.pos;
                    self.pos += 1;
                    self.maybe_directive(start, body_start, body_end);
                }
                (b'(', b'*') => {
                    self.pos += 2;
                    let body_start = self.pos;
                    while !(self.peek() == b'*' && self.peek_at(1) == b')') {
                        if self.pos >= self.src.len() {
                            return Err(self.error(start, "unterminated `(*` comment"));
                        }
                        self.pos += 1;
                    }
                    let body_end = self.pos;
                    self.pos += 2;
                    self.maybe_directive(start, body_start, body_end);
                }
                (b'/', b'/') => {
                    while self.pos < self.src.len() && self.peek() != b'\n' {
                        self.pos += 1;
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// A comment whose body starts with `$` is a compiler directive.
    fn maybe_directive(&mut self, start: usize, body_start: usize, body_end: usize) {
        let body = &self.src[body_start..body_end];
        if let Some(rest) = body.strip_prefix(b"$") {
            let text = String::from_utf8_lossy(rest).trim().to_string();
            self.push(TokenKind::Directive(text), start);
        }
    }

    fn identifier(&mut self) {
        let start = self.pos;
        while matches!(self.peek(), b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') {
            self.pos += 1;
        }
        // Identifiers are ASCII by construction, so this can't fail.
        let text = std::str::from_utf8(&self.src[start..self.pos])
            .unwrap()
            .to_string();
        let kind = match Keyword::from_lowercase(&text.to_ascii_lowercase()) {
            Some(kw) => TokenKind::Keyword(kw),
            None => TokenKind::Ident(text),
        };
        self.push(kind, start);
    }

    fn digits(&mut self) {
        while self.peek().is_ascii_digit() {
            self.pos += 1;
        }
    }

    fn number(&mut self) -> Result<(), Diagnostic> {
        let start = self.pos;
        self.digits();
        let mut is_real = false;
        // `1.5` is a real, but in `1..5` the dot belongs to `..`.
        if self.peek() == b'.' && self.peek_at(1).is_ascii_digit() {
            is_real = true;
            self.pos += 1;
            self.digits();
        }
        if matches!(self.peek(), b'e' | b'E') {
            let sign = matches!(self.peek_at(1), b'+' | b'-') as usize;
            if self.peek_at(1 + sign).is_ascii_digit() {
                is_real = true;
                self.pos += 1 + sign;
                self.digits();
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        let kind = if is_real {
            TokenKind::Real(
                text.parse()
                    .map_err(|_| self.error(start, "invalid real number"))?,
            )
        } else {
            TokenKind::Integer(
                text.parse()
                    .map_err(|_| self.error(start, "integer literal is too large"))?,
            )
        };
        self.push(kind, start);
        Ok(())
    }

    fn hex_number(&mut self) -> Result<(), Diagnostic> {
        let start = self.pos;
        self.pos += 1; // the `$`
        let digits_start = self.pos;
        while self.peek().is_ascii_hexdigit() {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.src[digits_start..self.pos]).unwrap();
        let value = i64::from_str_radix(text, 16)
            .map_err(|_| self.error(start, "invalid hexadecimal number"))?;
        self.push(TokenKind::Integer(value), start);
        Ok(())
    }

    /// Decide whether `^` starts a control-character literal (`^M` is
    /// `#13`) or is the pointer symbol (`^Node`, `p^`). Turbo Pascal treats it
    /// as a character when it can't be a dereference (the previous token isn't
    /// something you could dereference) and exactly one letter follows.
    fn caret_is_control_char(&self) -> bool {
        let after_value = matches!(
            self.tokens.last().map(|t| &t.kind),
            Some(TokenKind::Ident(_) | TokenKind::RParen | TokenKind::RBracket | TokenKind::Caret)
        );
        let letter = self.peek_at(1);
        let next = self.peek_at(2);
        !after_value
            && (letter.is_ascii_alphabetic() || matches!(letter, b'@' | b'[' | b'\\' | b']' | b'_'))
            && !(next.is_ascii_alphanumeric() || next == b'_')
    }

    /// A string literal: any run of `'quoted'` pieces, `#nn` codes and `^X`
    /// control characters with nothing in between.
    fn string(&mut self) -> Result<(), Diagnostic> {
        let start = self.pos;
        let mut bytes = Vec::new();
        loop {
            match self.peek() {
                b'\'' => {
                    self.pos += 1;
                    loop {
                        match self.peek() {
                            b'\'' if self.peek_at(1) == b'\'' => {
                                bytes.push(b'\'');
                                self.pos += 2;
                            }
                            b'\'' => {
                                self.pos += 1;
                                break;
                            }
                            b'\n' | b'\r' => {
                                return Err(
                                    self.error(start, "string literal is not closed on this line")
                                );
                            }
                            0 if self.pos >= self.src.len() => {
                                return Err(self.error(start, "unterminated string literal"));
                            }
                            c => {
                                bytes.push(c);
                                self.pos += 1;
                            }
                        }
                    }
                }
                b'#' => {
                    let hash = self.pos;
                    self.pos += 1;
                    let value = if self.peek() == b'$' {
                        self.pos += 1;
                        let digits_start = self.pos;
                        while self.peek().is_ascii_hexdigit() {
                            self.pos += 1;
                        }
                        u32::from_str_radix(
                            std::str::from_utf8(&self.src[digits_start..self.pos]).unwrap(),
                            16,
                        )
                    } else {
                        let digits_start = self.pos;
                        self.digits();
                        std::str::from_utf8(&self.src[digits_start..self.pos])
                            .unwrap()
                            .parse::<u32>()
                    };
                    match value {
                        Ok(v) if v <= 255 => bytes.push(v as u8),
                        _ => {
                            return Err(self.error(hash, "character code after `#` must be 0..255"));
                        }
                    }
                }
                b'^' if self.caret_is_control_char() || self.pos > start => {
                    // `^A` is 1, `^M` is 13: the letter's code with bit 6 cleared.
                    let letter = self.peek_at(1).to_ascii_uppercase();
                    if !(b'@'..=b'_').contains(&letter) {
                        break;
                    }
                    bytes.push(letter ^ 0x40);
                    self.pos += 2;
                }
                _ => break,
            }
        }
        self.push(TokenKind::Str(bytes), start);
        Ok(())
    }

    fn punctuation(&mut self) -> Result<(), Diagnostic> {
        use TokenKind::*;
        let start = self.pos;
        // Two-character tokens first, so `:=` wins over `:`.
        let two = match (self.peek(), self.peek_at(1)) {
            (b':', b'=') => Some(Assign),
            (b'<', b'>') => Some(NotEq),
            (b'<', b'=') => Some(LtEq),
            (b'>', b'=') => Some(GtEq),
            (b'.', b'.') => Some(DotDot),
            // `(.` and `.)` are old alternative spellings of `[` and `]`.
            (b'(', b'.') => Some(LBracket),
            (b'.', b')') => Some(RBracket),
            _ => None,
        };
        if let Some(kind) = two {
            self.pos += 2;
            self.push(kind, start);
            return Ok(());
        }
        let kind = match self.peek() {
            b'+' => Plus,
            b'-' => Minus,
            b'*' => Star,
            b'/' => Slash,
            b'=' => Eq,
            b'<' => Lt,
            b'>' => Gt,
            b'.' => Dot,
            b',' => Comma,
            b';' => Semi,
            b':' => Colon,
            b'(' => LParen,
            b')' => RParen,
            b'[' => LBracket,
            b']' => RBracket,
            b'^' => Caret,
            b'@' => At,
            other => {
                self.pos += 1;
                return Err(self.error(
                    start,
                    format!("unexpected character `{}`", other.escape_ascii()),
                ));
            }
        };
        self.pos += 1;
        self.push(kind, start);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    /// Tokenize and drop the spans and the final `Eof`, for compact asserts.
    fn kinds(src: &str) -> Vec<TokenKind> {
        let mut tokens: Vec<TokenKind> = tokenize(src.as_bytes())
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect();
        assert_eq!(tokens.pop(), Some(Eof));
        tokens
    }

    fn ident(s: &str) -> TokenKind {
        Ident(s.to_string())
    }

    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(
            kinds("BEGIN Begin begin End"),
            vec![
                Keyword(crate::token::Keyword::Begin),
                Keyword(crate::token::Keyword::Begin),
                Keyword(crate::token::Keyword::Begin),
                Keyword(crate::token::Keyword::End)
            ]
        );
    }

    #[test]
    fn identifiers_keep_their_spelling() {
        assert_eq!(
            kinds("GridT gridT _x1"),
            vec![ident("GridT"), ident("gridT"), ident("_x1")]
        );
    }

    #[test]
    fn directives_like_forward_are_identifiers() {
        assert_eq!(kinds("forward"), vec![ident("forward")]);
    }

    #[test]
    fn numbers() {
        assert_eq!(
            kinds("42 $FF 1.5 2e3 1.5E-2"),
            vec![
                Integer(42),
                Integer(255),
                Real(1.5),
                Real(2000.0),
                Real(0.015)
            ]
        );
    }

    #[test]
    fn range_dots_are_not_a_real() {
        assert_eq!(kinds("1..5"), vec![Integer(1), DotDot, Integer(5)]);
        assert_eq!(
            kinds("a[1..maxcol]"),
            vec![
                ident("a"),
                LBracket,
                Integer(1),
                DotDot,
                ident("maxcol"),
                RBracket
            ]
        );
    }

    #[test]
    fn strings() {
        assert_eq!(kinds("'it''s'"), vec![Str(b"it's".to_vec())]);
        assert_eq!(kinds("''"), vec![Str(vec![])]);
        assert_eq!(kinds("'ab'#13#10'cd'"), vec![Str(b"ab\r\ncd".to_vec())]);
        assert_eq!(kinds("#7"), vec![Str(vec![7])]);
        assert_eq!(kinds("#$41"), vec![Str(b"A".to_vec())]);
    }

    #[test]
    fn caret_characters_vs_pointers() {
        assert_eq!(
            kinds("x := ^M;"),
            vec![ident("x"), Assign, Str(vec![13]), Semi]
        );
        assert_eq!(
            kinds("p^.next"),
            vec![ident("p"), Caret, Dot, ident("next")]
        );
        assert_eq!(
            kinds("T = ^Node;"),
            vec![ident("T"), Eq, Caret, ident("Node"), Semi]
        );
    }

    #[test]
    fn comments_and_directives() {
        assert_eq!(
            kinds("a { one } b (* two *) c // three\n d {$I-} e"),
            vec![
                ident("a"),
                ident("b"),
                ident("c"),
                ident("d"),
                Directive("I-".to_string()),
                ident("e")
            ]
        );
    }

    #[test]
    fn operators() {
        assert_eq!(
            kinds(":= <> <= >= < > = + - * / .. . , ; : ( ) [ ] ^ @"),
            vec![
                Assign, NotEq, LtEq, GtEq, Lt, Gt, Eq, Plus, Minus, Star, Slash, DotDot, Dot,
                Comma, Semi, Colon, LParen, RParen, LBracket, RBracket, Caret, At
            ]
        );
    }

    #[test]
    fn dos_line_endings_and_ctrl_z() {
        assert_eq!(kinds("a\r\nb\r\n\x1a"), vec![ident("a"), ident("b")]);
    }

    #[test]
    fn errors() {
        assert!(tokenize(b"'abc").is_err());
        assert!(tokenize(b"{ never closed").is_err());
        assert!(tokenize(b"a ? b").is_err());
        assert!(tokenize(b"#300").is_err());
    }
}
