//! Source positions and error messages.

use std::fmt;

/// A range of bytes in a source file, `start..end`.
///
/// Spans are tiny and copied everywhere, so the type derives `Copy`: passing
/// one around duplicates the two numbers instead of moving ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

/// An error found while transpiling, tied to a place in the Pascal source.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic { span, message: message.into() }
    }

    /// Render as `file:line:col: error: message`, followed by the offending
    /// source line and a caret under the error, like rustc does.
    pub fn render(&self, file_name: &str, source: &[u8]) -> String {
        let (line, col) = line_col(source, self.span.start);
        let line_text = source_line(source, self.span.start);
        let width = (self.span.end.saturating_sub(self.span.start)).clamp(1, line_text.len().max(1));
        format!(
            "{file_name}:{line}:{col}: error: {}\n{line_text}\n{}{}",
            self.message,
            " ".repeat(col - 1),
            "^".repeat(width),
        )
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at byte {})", self.message, self.span.start)
    }
}

/// 1-based line and column of byte `offset`.
pub fn line_col(source: &[u8], offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.iter().filter(|&&b| b == b'\n').count() + 1;
    let line_start = before.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    (line, offset - line_start + 1)
}

/// The text of the line containing byte `offset`, without its line ending.
fn source_line(source: &[u8], offset: usize) -> String {
    let offset = offset.min(source.len());
    let start = source[..offset].iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let end = source[offset..].iter().position(|&b| b == b'\n').map_or(source.len(), |i| offset + i);
    // `from_utf8_lossy` replaces any non-UTF-8 byte (Pascal sources may be
    // code page 437) with `?`-like replacement characters instead of failing.
    String::from_utf8_lossy(&source[start..end]).trim_end_matches('\r').to_string()
}
