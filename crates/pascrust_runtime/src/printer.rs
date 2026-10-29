//! Turbo Pascal's `Printer` unit: `Lst`, a text file already open on the
//! printer, so `Writeln(Lst, 'Total: ', total)` prints a line.
//!
//! There's no printer port to write to, so where the output goes is a
//! setting, `PASCRUST_PRINTER`:
//!
//! - unset: appended to `printer.txt` in the directory the program started in;
//! - a file name: appended to that file;
//! - `|command`: piped to a shell command, such as `|lp` to really print.
//!
//! The destination is opened the first time something is printed, so a
//! program that never prints creates nothing. Each `Write(Lst, ...)` goes
//! straight to the file or command, unbuffered, as TP sent each byte to
//! LPT1, so nothing printed is lost even if the program is killed. When the
//! program ends (normally, by `Halt`, a runtime error or Ctrl-C) a command's
//! input is closed and we wait for it to finish, so the whole job is sent.
//! (If the process is killed outright, say by a signal Crt doesn't catch,
//! the command gets what was printed but isn't waited for.)

use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;

/// Where printed text goes.
enum Destination {
    File(std::fs::File),
    Command { child: Child, stdin: ChildStdin },
}

// A `Mutex` (rather than a thread-local) because the program prints from
// its worker thread but the printer is closed when the program ends, which
// may happen on the main thread (see `run`). `Mutex::new` is a `const fn`,
// so the mutex can be a plain `static`.
static PRINTER: Mutex<Option<Destination>> = Mutex::new(None);

/// Pascal `Lst`. It holds nothing itself: all printers share one
/// destination, as all of a PC's programs shared LPT1.
pub struct Lst;

impl Lst {
    /// Text for `Write(Lst, ...)`; used by the `fwrite!` macro.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        // A poisoned lock only means another thread panicked while printing;
        // the destination itself is still fine to use.
        let mut printer = PRINTER.lock().unwrap_or_else(|e| e.into_inner());
        let destination = printer.get_or_insert_with(|| open(&setting()));
        let ok = match destination {
            Destination::File(file) => file.write_all(bytes).is_ok(),
            Destination::Command { stdin, .. } => stdin.write_all(bytes).is_ok(),
        };
        if !ok {
            crate::rte(160, "Device write fault: can't write to the printer");
        }
    }

    /// Pascal `Flush(Lst)`.
    pub fn flush(&mut self) {
        let mut printer = PRINTER.lock().unwrap_or_else(|e| e.into_inner());
        match printer.as_mut() {
            Some(Destination::File(file)) => {
                let _ = file.flush();
            }
            Some(Destination::Command { stdin, .. }) => {
                let _ = stdin.flush();
            }
            None => {}
        }
    }
}

/// The `PASCRUST_PRINTER` setting, or the default file.
fn setting() -> String {
    std::env::var("PASCRUST_PRINTER").unwrap_or_else(|_| "printer.txt".to_string())
}

/// Open the destination a setting names: `|command` or a file name.
fn open(setting: &str) -> Destination {
    if let Some(command) = setting.strip_prefix('|') {
        let spawned = shell(command.trim()).stdin(Stdio::piped()).spawn();
        match spawned {
            Ok(mut child) => {
                let stdin = child.stdin.take().expect("stdin was piped");
                Destination::Command { child, stdin }
            }
            Err(e) => crate::rte(160, format!("Device write fault: can't run the printer command `{command}`: {e}")),
        }
    } else {
        // A relative name is relative to where the program started, like
        // the DOS drives (see `dos_path`).
        let path = crate::system::start_dir().join(setting);
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => Destination::File(file),
            Err(e) => crate::rte(160, format!("Device write fault: can't open {}: {e}", path.display())),
        }
    }
}

/// A command run by the system's shell, so `|lp -d office` works.
fn shell(command: &str) -> Command {
    let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    let mut cmd = Command::new(shell);
    cmd.arg(flag).arg(command);
    cmd
}

/// Finish printing when the program ends: flush the file, or close the
/// command's input and wait for it, so the whole job reaches the printer.
pub(crate) fn close() {
    let destination = PRINTER.lock().unwrap_or_else(|e| e.into_inner()).take();
    match destination {
        Some(Destination::File(mut file)) => {
            let _ = file.flush();
        }
        Some(Destination::Command { mut child, stdin }) => {
            // Dropping `stdin` closes the pipe: the command sees the end of
            // its input and can finish.
            drop(stdin);
            let _ = child.wait();
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_destination_gets_everything_printed() {
        let out = std::env::temp_dir().join(format!("pascrust_printer_{}.txt", std::process::id()));
        let mut destination = open(&format!("|cat > '{}'", out.display()));
        if let Destination::Command { stdin, .. } = &mut destination {
            stdin.write_all(b"page one\n\x0c").unwrap();
        }
        *PRINTER.lock().unwrap() = Some(destination);
        close();
        assert_eq!(std::fs::read(&out).unwrap(), b"page one\n\x0c");
        std::fs::remove_file(&out).unwrap();
    }
}
