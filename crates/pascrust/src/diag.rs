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

/// An error (or a warning) found while transpiling, tied to a place in the
/// Pascal source.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    /// A warning doesn't stop the translation.
    pub warning: bool,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic { span, message: message.into(), warning: false }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic { warning: true, ..Diagnostic::new(span, message) }
    }

    /// Render as `file:line:col: error: message`, followed by the offending
    /// source line and a caret under the error, like rustc does.
    pub fn render(&self, file_name: &str, source: &[u8]) -> String {
        let (line, col) = line_col(source, self.span.start);
        let line_text = source_line(source, self.span.start);
        let width = (self.span.end.saturating_sub(self.span.start)).clamp(1, line_text.len().max(1));
        format!(
            "{file_name}:{line}:{col}: {}: {}\n{line_text}\n{}{}",
            if self.warning { "warning" } else { "error" },
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

/// All the source files of a program, laid end to end in one big "address
/// space". Each file starts at its own *base* offset, so a [`Span`] alone
/// identifies both the file and the position in it. (rustc's own
/// `SourceMap` works the same way.)
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

#[derive(Debug)]
pub struct SourceFile {
    pub name: String,
    pub base: usize,
    pub text: Vec<u8>,
}

impl SourceMap {
    pub fn new() -> SourceMap {
        SourceMap::default()
    }

    /// Add a file and return its base offset.
    pub fn add(&mut self, name: impl Into<String>, text: Vec<u8>) -> usize {
        // Leave a gap of one byte so that an end-of-file span of one file
        // never touches the next file.
        let base = self.files.last().map_or(0, |f| f.base + f.text.len() + 1);
        self.files.push(SourceFile { name: name.into(), base, text });
        base
    }

    /// The index of the file containing `offset`.
    pub fn file_index(&self, offset: usize) -> usize {
        // `partition_point` is a binary search: the number of files whose
        // base is <= offset, so the last of those contains it.
        self.files.partition_point(|f| f.base <= offset).saturating_sub(1)
    }

    pub fn file(&self, index: usize) -> &SourceFile {
        &self.files[index]
    }

    /// The source text of a span.
    pub fn slice(&self, span: Span) -> &[u8] {
        let file = &self.files[self.file_index(span.start)];
        let start = (span.start - file.base).min(file.text.len());
        let end = (span.end.saturating_sub(file.base)).clamp(start, file.text.len());
        &file.text[start..end]
    }

    /// Render a diagnostic with the right file name and line.
    pub fn render(&self, d: &Diagnostic) -> String {
        if self.files.is_empty() {
            return d.message.clone();
        }
        let file = &self.files[self.file_index(d.span.start)];
        // `..d.clone()` fills in the fields not named here from `d`.
        let local =
            Diagnostic { span: Span::new(d.span.start - file.base, d.span.end.saturating_sub(file.base)), ..d.clone() };
        local.render(&file.name, &file.text)
    }
}
