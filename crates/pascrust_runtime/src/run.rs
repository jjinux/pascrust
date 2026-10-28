//! Program startup, runtime errors and `Halt`.
//!
//! Turbo Pascal stops a program with "Runtime error 201 at 0000:0123." when
//! something goes wrong (a range check fails, a nil pointer is used, a file is
//! missing, ...). We model every runtime error as a Rust panic carrying a
//! [`RuntimeError`] value, and [`run`] catches it once at the top. Using
//! panics, rather than returning `Result` from every function, keeps the
//! generated code free of `?` on every array index and `writeln`.

use std::any::Any;
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
    panic::panic_any(RuntimeError {
        code,
        message: message.into(),
    })
}

/// Pascal `Halt(code)`: flush output and end the program immediately.
pub fn halt(code: i32) -> ! {
    crate::io::flush_stdout();
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

    let worker = std::thread::Builder::new()
        .name("pascal".to_string())
        .stack_size(256 * 1024 * 1024)
        .spawn(program)
        .expect("failed to start the Pascal program thread");

    // `join` returns `Err(payload)` if the thread panicked; this is how we
    // "catch" the panic without needing `catch_unwind` ourselves.
    let result = worker.join();
    crate::io::flush_stdout();
    if let Err(payload) = result {
        let err = to_runtime_error(payload);
        eprintln!("Runtime error {}: {}", err.code, err.message);
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
    let code = if message.contains("divide by zero")
        || message.contains("remainder with a divisor of zero")
    {
        200
    } else if message.contains("already borrowed") || message.contains("already mutably borrowed") {
        204
    } else {
        255
    };
    RuntimeError { code, message }
}
