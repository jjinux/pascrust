//! Program startup, runtime errors and `Halt`.
//!
//! Turbo Pascal stops a program with "Runtime error 201 at 0000:0123." when
//! something goes wrong (a range check fails, a nil pointer is used, a file is
//! missing, ...). We model every runtime error as a Rust panic carrying a
//! [`RuntimeError`] value, and [`run`] catches it once at the top. Using
//! panics, rather than returning `Result` from every function, keeps the
//! generated code free of `?` on every array index and `writeln`.

use std::any::Any;
use std::cell::Cell;
use std::panic;

/// The payload of a Pascal runtime error panic.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// The Turbo Pascal error number, e.g. 201 for "Range check error".
    pub code: i32,
    pub message: String,
}

/// Raise Pascal runtime error `code`. Never returns.
///
/// The return type `!` is Rust's "never" type: it tells the compiler this
/// function diverges, so `let x: i16 = if ok { 1 } else { rte(201, "...") };`
/// type-checks.
pub fn rte(code: i32, message: impl Into<String>) -> ! {
    // `panic_any` panics with an arbitrary value instead of a string, so
    // `run` can downcast the payload and recover the error code.
    panic::panic_any(RuntimeError { code, message: message.into() })
}

/// Pascal `RunError(code)`: stop with runtime error `code`, as if the
/// runtime had found one. (Declared to return `()`, like `halt`, so
/// statements after it don't warn.)
pub fn run_error(code: impl Into<i32>) {
    rte(code.into(), "")
}

thread_local! {
    /// Turbo Pascal's `InOutRes`: the code of the last I/O error made with
    /// `{$I-}`, until `IOResult` reads it.
    static IO_RESULT: Cell<i16> = const { Cell::new(0) };
}

/// An I/O call compiled with `{$I-}`: `io_unchecked(|| f.reset())`. An I/O
/// error (code below 200) doesn't stop the program; its code is kept for
/// `IOResult`, and the call returns a default value (`false` for `Eof`).
/// As in TP, while an error is waiting to be read, further unchecked I/O
/// calls do nothing.
pub fn io_unchecked<R: Default>(f: impl FnOnce() -> R) -> R {
    if IO_RESULT.get() != 0 {
        return R::default();
    }
    // `catch_unwind` stops a panic (our runtime errors are panics, see
    // `rte`) and returns it as an `Err`. `AssertUnwindSafe` promises Rust
    // that nothing is left half-changed when it does, which holds here: an
    // I/O error is raised before the file's state changes.
    match panic::catch_unwind(panic::AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            let code = payload.downcast_ref::<RuntimeError>().map(|error| error.code);
            match code {
                Some(code) if code < 200 => {
                    IO_RESULT.set(code as i16);
                    R::default()
                }
                // Range errors and the like still stop the program.
                _ => panic::resume_unwind(payload),
            }
        }
    }
}

/// Pascal `IOResult`: the code of the last I/O error under `{$I-}` (0 if
/// there was none), which it then clears.
pub fn io_result() -> i16 {
    IO_RESULT.replace(0)
}

/// Pascal `Halt(code)`: flush output and end the program immediately.
///
/// This never returns, but it's declared to return `()` rather than `!`:
/// Pascal programs sometimes have statements after a `Halt`, and with `!`
/// Rust would warn that they're unreachable.
pub fn halt(code: i32) {
    #[cfg(feature = "graph")]
    crate::graph::save_if_requested();
    crate::io::shutdown();
    std::process::exit(code)
}

/// Run a translated Pascal program. Generated `main` functions call this.
///
/// The program runs on a thread with a large stack (Pascal programs often keep
/// big arrays in globals, which pascrust turns into locals of `pascal_main`),
/// and any panic is translated into a Turbo Pascal style runtime error message
/// and exit code.
pub fn run(program: fn()) {
    // Silence Rust's default "thread panicked at ..." message for Pascal
    // runtime errors (we print our own), but keep it for genuine bugs, where
    // the Rust location is useful for debugging pascrust itself.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if info.payload().downcast_ref::<RuntimeError>().is_none() {
            default_hook(info);
        }
    }));

    // Remember where we started: unmapped DOS drives (`A:\`) mean this
    // directory, even after the program uses `ChDir`.
    crate::system::start_dir();

    let worker = std::thread::Builder::new()
        .name("pascal".to_string())
        .stack_size(256 * 1024 * 1024)
        .spawn(program)
        .expect("failed to start the Pascal program thread");

    // A `Graph` program's window has to be run from this, the main, thread.
    #[cfg(feature = "graph")]
    crate::graph::window::main_loop(&worker);

    // `join` returns `Err(payload)` if the thread panicked; this is how we
    // "catch" the panic without needing `catch_unwind` ourselves.
    let result = worker.join();
    #[cfg(feature = "graph")]
    crate::graph::save_if_requested();
    crate::io::shutdown();
    if let Ok(()) = result {
        // A program that ends normally exits with `ExitCode` (usually 0).
        let code = crate::system::exit_code();
        if code != 0 {
            std::process::exit(i32::from(code));
        }
    }
    if let Err(payload) = result {
        let err = to_runtime_error(payload);
        if err.message.is_empty() {
            eprintln!("Runtime error {}", err.code);
        } else {
            eprintln!("Runtime error {}: {}", err.code, err.message);
        }
        std::process::exit(err.code);
    }
}

/// Turn any panic payload into a [`RuntimeError`]. Panics raised by Rust
/// itself (rather than by [`rte`]) are mapped to the closest Pascal error.
fn to_runtime_error(payload: Box<dyn Any + Send>) -> RuntimeError {
    // `downcast` tries to recover the concrete type hidden behind `dyn Any`.
    let payload = match payload.downcast::<RuntimeError>() {
        Ok(err) => return *err,
        Err(other) => other,
    };
    let message = if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown error".to_string()
    };
    let code = if message.contains("divide by zero") || message.contains("remainder with a divisor of zero") {
        200
    } else if message.contains("already borrowed") || message.contains("already mutably borrowed") {
        204
    } else {
        255
    };
    RuntimeError { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_unchecked_keeps_the_error_for_io_result() {
        let eof: bool = io_unchecked(|| rte(2, "File not found"));
        assert!(!eof);
        // While the error waits, other unchecked I/O is skipped.
        let ran = io_unchecked(|| true);
        assert!(!ran);
        assert_eq!(io_result(), 2);
        assert_eq!(io_result(), 0);
        // A range check isn't an I/O error: it still stops the program.
        assert!(panic::catch_unwind(|| io_unchecked::<()>(|| rte(201, "Range check error"))).is_err());
    }
}
