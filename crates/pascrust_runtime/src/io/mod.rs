//! Input and output: Pascal's `Write`/`Writeln`, `Read`/`Readln`, and text
//! and typed files.

mod files;
mod read;
mod write;

pub use files::{FileRecord, PFile, PText};

#[cfg(feature = "crt")]
pub(crate) use read::read_key_plain;
pub use read::{LineReader, PasRead, eof, eoln, read_into, readln_end};
pub use write::{Formatted, PasWrite, w, wd};

use std::io::Write;

/// Send bytes to standard output. All Pascal output funnels through here.
///
/// On a terminal, code page 437 characters are translated to UTF-8 so that
/// box drawing and accented letters look right; and while `Crt` has the
/// terminal in raw mode, `\n` must become `\r\n` because the terminal no
/// longer returns to the start of the line by itself. When output goes to a
/// file or pipe, the bytes are written exactly as the program produced them.
pub fn write_stdout(bytes: &[u8]) {
    let mut out = std::io::stdout().lock();
    let mut bytes = std::borrow::Cow::Borrowed(bytes);
    if raw_terminal() {
        let mut converted = Vec::with_capacity(bytes.len());
        for &b in bytes.iter() {
            if b == b'\n' {
                converted.push(b'\r');
            }
            converted.push(b);
        }
        bytes = converted.into();
    }
    if is_terminal() && bytes.iter().any(|&b| b >= 128) {
        bytes = crate::cp437::to_utf8(&bytes).into();
    }
    // A failed write to stdout (e.g. a closed pipe) is ignored, as in TP.
    let _ = out.write_all(&bytes);
}

fn is_terminal() -> bool {
    use std::io::IsTerminal;
    // `OnceLock` runs the check once and caches the answer.
    static IS_TERMINAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *IS_TERMINAL.get_or_init(|| std::io::stdout().is_terminal())
}

/// Is `Crt` holding the terminal in raw mode?
pub(crate) fn raw_terminal() -> bool {
    #[cfg(feature = "crt")]
    {
        crate::crt::raw_mode_active()
    }
    #[cfg(not(feature = "crt"))]
    {
        false
    }
}

/// Restore the terminal (if `Crt` changed it) and flush output. Called on
/// every way out of the program.
pub(crate) fn shutdown() {
    flush_stdout();
    #[cfg(feature = "crt")]
    crate::crt::restore();
}

pub fn flush_stdout() {
    let _ = std::io::stdout().flush();
}
