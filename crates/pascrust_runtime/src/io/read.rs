//! `Read` and `Readln`, from standard input or a text file.
//!
//! Turbo Pascal reads text line by line. `Read(x)` for a number skips
//! blanks (including line breaks) and parses the number; `Read(s)` for a
//! string takes the rest of the current line; `Readln` then discards whatever
//! is left of the line.

use std::cell::RefCell;
use std::io::BufRead;

use crate::string::PString;

/// Where a [`LineReader`] gets its lines.
pub(crate) enum Source {
    Stdin,
    File(std::io::BufReader<std::fs::File>),
}

/// The current input line and how far into it we've read. Standard input and
/// every open text file each have one.
pub struct LineReader {
    source: Source,
    line: Vec<u8>,
    pos: usize,
    /// True once a line has been fetched and not yet finished by `Readln`.
    have_line: bool,
    /// True if the current line is the last one and has no line break after
    /// it: then the end of the line is also the end of the file.
    unterminated: bool,
    eof: bool,
}

thread_local! {
    // `RefCell` lets us mutate the input state through a shared global; it
    // checks at runtime that no two borrows overlap.
    static INPUT: RefCell<LineReader> = RefCell::new(LineReader::new(Source::Stdin));
}

impl LineReader {
    pub(crate) fn new(source: Source) -> LineReader {
        LineReader { source, line: Vec::new(), pos: 0, have_line: false, unterminated: false, eof: false }
    }

    /// Make sure there is a line to read from. Returns false at end of input.
    fn fill(&mut self) -> bool {
        if self.have_line {
            return true;
        }
        if self.eof {
            return false;
        }
        self.line.clear();
        self.pos = 0;
        let result = match &mut self.source {
            Source::Stdin => {
                // Prompts written with `Write` (no newline) must appear
                // before we wait for input.
                crate::io::flush_stdout();
                // In raw mode the terminal doesn't echo or edit lines; Crt
                // does it.
                #[cfg(feature = "crt")]
                if crate::io::raw_terminal() {
                    return match crate::crt::read_line_raw() {
                        Some(line) => {
                            self.line = line;
                            self.unterminated = false;
                            self.have_line = true;
                            true
                        }
                        None => {
                            self.eof = true;
                            false
                        }
                    };
                }
                std::io::stdin().lock().read_until(b'\n', &mut self.line)
            }
            Source::File(file) => file.read_until(b'\n', &mut self.line),
        };
        match result {
            Ok(0) | Err(_) => {
                self.eof = true;
                false
            }
            Ok(_) => {
                // Strip the line ending (`\n` or DOS `\r\n`).
                self.unterminated = self.line.last() != Some(&b'\n');
                if !self.unterminated {
                    self.line.pop();
                }
                if self.line.last() == Some(&b'\r') {
                    self.line.pop();
                }
                self.have_line = true;
                true
            }
        }
    }

    fn at_eoln(&self) -> bool {
        self.pos >= self.line.len()
    }

    /// Skip blanks and line breaks before a number.
    fn skip_blanks(&mut self) -> bool {
        loop {
            if !self.fill() {
                return false;
            }
            while !self.at_eoln() && matches!(self.line[self.pos], b' ' | b'\t') {
                self.pos += 1;
            }
            if !self.at_eoln() {
                return true;
            }
            self.have_line = false;
        }
    }

    /// The next whitespace-delimited word, for numbers.
    fn word(&mut self) -> String {
        if !self.skip_blanks() {
            return String::new();
        }
        let start = self.pos;
        while !self.at_eoln() && !matches!(self.line[self.pos], b' ' | b'\t') {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.line[start..self.pos]).into_owned()
    }

    /// The next character, `#13` at the end of a line, or `None` at the end
    /// of the input.
    ///
    /// (On DOS, TP returned `#13` and then `#10` for a line break, as they
    /// were both in the file; Free Pascal on Unix returns just `#10`. We
    /// return just `#13`, which is what TP's own `Eoln`-style loops expect.)
    fn next_char(&mut self) -> Option<u8> {
        if self.eof() {
            return None;
        }
        if self.at_eoln() {
            self.have_line = false;
            return Some(13);
        }
        self.pos += 1;
        Some(self.line[self.pos - 1])
    }

    /// Finish a `Readln`: skip the rest of the current line (or, if nothing
    /// has been read from it yet, a whole line).
    pub(crate) fn end_line(&mut self) {
        self.fill();
        self.have_line = false;
    }

    pub(crate) fn eof(&mut self) -> bool {
        !self.fill() || (self.unterminated && self.at_eoln())
    }

    pub(crate) fn eoln(&mut self) -> bool {
        !self.fill() || self.at_eoln()
    }
}

/// A type that `Read` can read.
pub trait PasRead {
    #[doc(hidden)]
    fn pas_read(&mut self, input: &mut LineReader);
}

macro_rules! impl_read_number {
    ($($t:ty),*) => {
        $( impl PasRead for $t {
            fn pas_read(&mut self, input: &mut LineReader) {
                let word = input.word();
                if word.is_empty() {
                    // TP reads 0 at end of file.
                    *self = Default::default();
                    return;
                }
                match word.parse::<$t>() {
                    Ok(v) => *self = v,
                    Err(_) => crate::rte(106, format!("Invalid numeric format: {word:?}")),
                }
            }
        } )*
    };
}
impl_read_number!(i8, i16, i32, i64, u16, f64);

/// Reading a `Char` takes exactly one character. At the end of a line it
/// gives `#13` (carriage return), as on DOS, and moves to the next line.
impl PasRead for u8 {
    fn pas_read(&mut self, input: &mut LineReader) {
        // 26 is Ctrl-Z, the DOS end-of-file character.
        *self = input.next_char().unwrap_or(26);
    }
}

/// Reading a string takes the rest of the current line.
impl PasRead for PString {
    fn pas_read(&mut self, input: &mut LineReader) {
        *self = if input.fill() {
            let s = PString::from_bytes(&input.line[input.pos..]);
            input.pos = input.line.len();
            s
        } else {
            PString::new()
        };
    }
}

/// Read one value from standard input; used by the `pread!` and `preadln!`
/// macros.
pub fn read_into<T: PasRead>(target: &mut T) {
    INPUT.with_borrow_mut(|input| target.pas_read(input));
}

/// Finish a `Readln` from standard input.
pub fn readln_end() {
    INPUT.with_borrow_mut(LineReader::end_line);
}

/// `ReadKey` when there's no terminal to read keys from: take the next
/// character of standard input, with the end of a line reading as Enter
/// (`#13`). At the end of the input, stop the program, since a scripted
/// program waiting for a key would otherwise wait forever.
#[cfg(feature = "crt")]
pub(crate) fn read_key_plain() -> u8 {
    match INPUT.with_borrow_mut(LineReader::next_char) {
        Some(key) => key,
        None => {
            eprintln!("[pascrust] ReadKey: no more input, stopping the program");
            crate::halt(0);
            unreachable!()
        }
    }
}

/// Pascal `Eof` for standard input.
pub fn eof() -> bool {
    INPUT.with_borrow_mut(LineReader::eof)
}

/// Pascal `Eoln` for standard input.
pub fn eoln() -> bool {
    INPUT.with_borrow_mut(LineReader::eoln)
}

/// Pascal `Read(a, b, ...)` from standard input. Arguments are `&mut`
/// references to the variables to fill.
#[macro_export]
macro_rules! pread {
    ($($arg:expr),* $(,)?) => {{
        $( $crate::io::read_into($arg); )*
    }};
}

/// Pascal `Readln(a, b, ...)`: like [`pread!`], then skip to the next line.
#[macro_export]
macro_rules! preadln {
    ($($arg:expr),* $(,)?) => {{
        $( $crate::io::read_into($arg); )*
        $crate::io::readln_end();
    }};
}
