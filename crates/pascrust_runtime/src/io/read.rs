//! `Read` and `Readln` from standard input.
//!
//! Turbo Pascal reads text line by line. `Read(x)` for a number skips
//! blanks (including line breaks) and parses the number; `Read(s)` for a
//! string takes the rest of the current line; `Readln` then discards whatever
//! is left of the line.

use std::cell::RefCell;
use std::io::BufRead;

use crate::string::PString;

/// The current input line and how far into it we've read.
#[derive(Default)]
struct Input {
    line: Vec<u8>,
    pos: usize,
    /// True once a line has been fetched and not yet finished by `Readln`.
    have_line: bool,
    eof: bool,
}

thread_local! {
    // `RefCell` lets us mutate the input state through a shared global; it
    // checks at runtime that no two borrows overlap.
    static INPUT: RefCell<Input> = RefCell::new(Input::default());
}

impl Input {
    /// Make sure there is a line to read from. Returns false at end of input.
    fn fill(&mut self) -> bool {
        if self.have_line {
            return true;
        }
        if self.eof {
            return false;
        }
        // Prompts written with `Write` (no newline) must appear before we
        // wait for input.
        crate::io::flush_stdout();
        self.line.clear();
        self.pos = 0;
        match std::io::stdin().lock().read_until(b'\n', &mut self.line) {
            Ok(0) | Err(_) => {
                self.eof = true;
                false
            }
            Ok(_) => {
                // Strip the line ending (`\n` or DOS `\r\n`).
                if self.line.last() == Some(&b'\n') {
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
}

/// A type that `Read` can read.
pub trait PasRead {
    fn pas_read(&mut self);
}

macro_rules! impl_read_number {
    ($($t:ty),*) => {
        $( impl PasRead for $t {
            fn pas_read(&mut self) {
                let word = INPUT.with_borrow_mut(|input| input.word());
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
    fn pas_read(&mut self) {
        *self = INPUT.with_borrow_mut(|input| {
            if !input.fill() {
                return 26; // Ctrl-Z, the DOS end-of-file character
            }
            if input.at_eoln() {
                input.have_line = false;
                return 13;
            }
            input.pos += 1;
            input.line[input.pos - 1]
        });
    }
}

/// Reading a string takes the rest of the current line.
impl PasRead for PString {
    fn pas_read(&mut self) {
        *self = INPUT.with_borrow_mut(|input| {
            if !input.fill() {
                return PString::new();
            }
            let s = PString::from_bytes(&input.line[input.pos..]);
            input.pos = input.line.len();
            s
        });
    }
}

/// Read one value; used by the `pread!` and `preadln!` macros.
pub fn read_into<T: PasRead>(target: &mut T) {
    target.pas_read();
}

/// Finish a `Readln`: skip the rest of the current line (or, if nothing has
/// been read from it yet, a whole line).
pub fn readln_end() {
    INPUT.with_borrow_mut(|input| {
        input.fill();
        input.have_line = false;
    });
}

/// Pascal `Eof` for standard input.
pub fn eof() -> bool {
    INPUT.with_borrow_mut(|input| !input.fill())
}

/// Pascal `Eoln` for standard input.
pub fn eoln() -> bool {
    INPUT.with_borrow_mut(|input| !input.fill() || input.at_eoln())
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
