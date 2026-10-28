//! The tokens produced by the lexer.

use crate::diag::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// An identifier, spelled as in the source. Pascal is case-insensitive,
    /// so comparisons go through [`TokenKind::is_ident`] or lowercase keys.
    Ident(String),
    Keyword(Keyword),
    Integer(i64),
    Real(f64),
    /// A string literal. Pascal has no separate character literal syntax:
    /// `'A'` is a string of length one, and sema decides whether it's used
    /// as a `Char`. Strings are bytes because Turbo Pascal strings are.
    Str(Vec<u8>),
    /// A compiler directive comment such as `{$I-}` or `{$M 16384,0,0}`,
    /// with the braces and `$` stripped.
    Directive(String),

    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Assign,
    Dot,
    DotDot,
    Comma,
    Semi,
    Colon,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Caret,
    At,
    Eof,
}

impl TokenKind {
    /// True if this is the identifier `name`, ignoring case. Used for words
    /// like `forward` that are only special in certain positions.
    pub fn is_ident(&self, name: &str) -> bool {
        matches!(self, TokenKind::Ident(s) if s.eq_ignore_ascii_case(name))
    }

    /// A short description for error messages ("expected `;`, found ...").
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(s) => format!("identifier `{s}`"),
            TokenKind::Keyword(k) => format!("`{}`", k.as_str()),
            TokenKind::Integer(n) => format!("number `{n}`"),
            TokenKind::Real(x) => format!("number `{x}`"),
            TokenKind::Str(s) => format!("string '{}'", String::from_utf8_lossy(s)),
            TokenKind::Directive(d) => format!("directive `{{${d}}}`"),
            TokenKind::Eof => "end of file".to_string(),
            other => format!("`{}`", other.punct_str()),
        }
    }

    fn punct_str(&self) -> &'static str {
        match self {
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Eq => "=",
            TokenKind::NotEq => "<>",
            TokenKind::Lt => "<",
            TokenKind::LtEq => "<=",
            TokenKind::Gt => ">",
            TokenKind::GtEq => ">=",
            TokenKind::Assign => ":=",
            TokenKind::Dot => ".",
            TokenKind::DotDot => "..",
            TokenKind::Comma => ",",
            TokenKind::Semi => ";",
            TokenKind::Colon => ":",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::Caret => "^",
            TokenKind::At => "@",
            _ => "?",
        }
    }
}

/// Generates the `Keyword` enum together with its string table, so the two
/// can never get out of sync. Each line is `Variant => "spelling"`.
macro_rules! keywords {
    ($($variant:ident => $text:literal),* $(,)?) => {
        /// Turbo Pascal's reserved words. Words such as `forward`, `external`
        /// or `virtual` are *not* reserved in TP (they're "directives"), so
        /// they're lexed as identifiers and recognized by the parser in
        /// context.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Keyword { $($variant),* }

        impl Keyword {
            pub fn as_str(self) -> &'static str {
                match self { $(Keyword::$variant => $text),* }
            }

            /// Look up a keyword; `word` must already be lowercase.
            pub fn from_lowercase(word: &str) -> Option<Keyword> {
                match word { $($text => Some(Keyword::$variant),)* _ => None }
            }
        }
    };
}

keywords! {
    And => "and", Array => "array", Asm => "asm", Begin => "begin",
    Case => "case", Const => "const", Constructor => "constructor",
    Destructor => "destructor", Div => "div", Do => "do", Downto => "downto",
    Else => "else", End => "end", File => "file", For => "for",
    Function => "function", Goto => "goto", If => "if",
    Implementation => "implementation", In => "in", Inherited => "inherited",
    Inline => "inline", Interface => "interface", Label => "label",
    Mod => "mod", Nil => "nil", Not => "not", Object => "object", Of => "of",
    Or => "or", Packed => "packed", Procedure => "procedure",
    Program => "program", Record => "record", Repeat => "repeat", Set => "set",
    Shl => "shl", Shr => "shr", String => "string", Then => "then", To => "to",
    Type => "type", Unit => "unit", Until => "until", Uses => "uses",
    Var => "var", While => "while", With => "with", Xor => "xor",
}
