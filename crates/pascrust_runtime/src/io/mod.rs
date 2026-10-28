//! Input and output: Pascal's `Write`/`Writeln` (and later `Read`/`Readln`
//! and files).

mod read;
mod write;

pub use read::{PasRead, eof, eoln, read_into, readln_end};
pub use write::{Formatted, PasWrite, w, wd};

use std::io::Write;

/// Send bytes to standard output. All Pascal output funnels through here so
/// that the `Crt` unit can later track the cursor position.
pub fn write_stdout(bytes: &[u8]) {
    let mut out = std::io::stdout().lock();
    // A failed write to stdout (e.g. a closed pipe) is ignored, as in TP.
    let _ = out.write_all(bytes);
}

pub fn flush_stdout() {
    let _ = std::io::stdout().flush();
}
