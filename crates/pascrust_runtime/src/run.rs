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
    /// After `Halt`, its exit code.
    pub code: i32,
    pub message: String,
    /// True if the Pascal code called `Halt` (inside [`call`]), rather than
    /// failing.
    pub halted: bool,
}

/// Raise Pascal runtime error `code`. Never returns.
///
/// The return type `!` is Rust's "never" type: it tells the compiler this
/// function diverges, so `let x: i16 = if ok { 1 } else { rte(201, "...") };`
/// type-checks.
pub fn rte(code: i32, message: impl Into<String>) -> ! {
    // Pascal code a Rust program calls without `call` gets the message too.
    quiet_runtime_errors();
    // `panic_any` panics with an arbitrary value instead of a string, so
    // `run` can downcast the payload and recover the error code.
    panic::panic_any(RuntimeError { code, message: message.into(), halted: false })
}

/// `{$Q+}`: an integer operation overflowed (`checked_add` and friends gave
/// `None`), so stop with runtime error 215, as Turbo Pascal does. Generic
/// over the result's type, so it fits `x.checked_add(y).unwrap_or_else(overflow)`.
pub fn overflow<T>() -> T {
    rte(215, "Arithmetic overflow error")
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
    // that nothing is left half-changed when it does. That holds here: a
    // failed I/O call leaves its file in a consistent state (a failed
    // `Reset` has closed the file first, as TP's does).
    UNCHECKED_DEPTH.set(UNCHECKED_DEPTH.get() + 1);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(f));
    UNCHECKED_DEPTH.set(UNCHECKED_DEPTH.get() - 1);
    match result {
        Ok(value) => value,
        Err(payload) => {
            // (Not `Halt` inside a `call`, which ends what `call` runs.)
            let code = payload.downcast_ref::<RuntimeError>().filter(|error| !error.halted).map(|error| error.code);
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
    // Inside `call`, a Rust program called this Pascal code: `Halt` comes
    // back to it as an `Err`, rather than ending its process.
    // (With `panic = "abort"` nothing could catch it, so `Halt` ends the
    // process, as in a Pascal program.)
    if CALL_DEPTH.get() > 0 && !cfg!(panic = "abort") {
        crate::io::flush_stdout();
        panic::panic_any(RuntimeError { code, message: "Halt".to_string(), halted: true });
    }
    #[cfg(feature = "graph")]
    crate::graph::save_if_requested();
    crate::io::shutdown();
    std::process::exit(code)
}

thread_local! {
    /// How many [`call`]s this thread is inside.
    static CALL_DEPTH: Cell<u32> = const { Cell::new(0) };
    /// How many [`io_unchecked`]s: an I/O error there is for `IOResult`,
    /// not a message.
    static UNCHECKED_DEPTH: Cell<u32> = const { Cell::new(0) };
    /// True while [`warn_unless_wrapping`] tries an overflow on purpose.
    static PROBING: Cell<bool> = const { Cell::new(false) };
}

/// Turbo Pascal's integer arithmetic wraps around (32767 + 1 is -32768).
/// A Pascal program's own Cargo.toml turns Rust's overflow checks off, but
/// a library is built with the Rust program's settings, where a debug
/// build has them on. A library's context calls this once with `probe`,
/// an addition compiled in the library (`i16::MAX + 1`), and says so on
/// stderr if it panicked.
pub fn warn_unless_wrapping(probe: fn() -> i16) {
    // With `panic = "abort"` a panic can't be caught, so there's no asking.
    if cfg!(panic = "abort") {
        return;
    }
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        quiet_runtime_errors();
        PROBING.set(true);
        let checked = panic::catch_unwind(probe).is_err();
        PROBING.set(false);
        if checked {
            eprintln!(
                "[PascRust] Rust's overflow checks are on for the Pascal code, so arithmetic Turbo Pascal \
                 wraps around (32767 + 1) will stop with a runtime error instead. Turn them off for it in \
                 your Cargo.toml (`overflow-checks = false`; see docs/embedding.md in PascRust)."
            );
        }
    });
}

/// Run a translated Pascal program. Generated `main` functions call this.
///
/// The program runs on a thread with a large stack (Pascal programs often keep
/// big arrays in globals, which PascRust turns into locals of `pascal_main`),
/// and any panic is translated into a Turbo Pascal style runtime error message
/// and exit code.
pub fn run(program: fn()) {
    quiet_runtime_errors();

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

/// Silence Rust's default "thread panicked at ..." message for Pascal
/// runtime errors (we report them ourselves), but keep it for genuine bugs,
/// where the Rust location is useful for debugging PascRust itself. Done
/// once: `Once` runs its closure the first time only.
fn quiet_runtime_errors() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            match info.payload().downcast_ref::<RuntimeError>() {
                // (`warn_unless_wrapping` overflows on purpose.)
                None if PROBING.get() => {}
                None => default_hook(info),
                // `run` (on its "pascal" thread) and `call` report it; outside
                // them (Pascal code a Rust program called directly), say what
                // happened before the panic stops the program.
                Some(err) => {
                    let in_run = std::thread::current().name() == Some("pascal");
                    // (An I/O error under `{$I-}` is for `IOResult`.)
                    let for_io_result = UNCHECKED_DEPTH.get() > 0 && err.code < 200 && !err.halted;
                    let caught = in_run || CALL_DEPTH.get() > 0 || for_io_result;
                    if !caught {
                        eprintln!("{err}");
                    } else if cfg!(panic = "abort") {
                        // With `panic = "abort"` nothing can catch it: the
                        // program ends right after this, so say why.
                        eprintln!("{err}");
                        let lost = if for_io_result {
                            "{$I-} can't keep it for IOResult"
                        } else if CALL_DEPTH.get() > 0 {
                            "`call` can't return it as an Err"
                        } else {
                            "it ends the program at once"
                        };
                        eprintln!(
                            "[PascRust] The program is built with panic = \"abort\", so {lost} (see \"Overflow checks\" in PascRust's docs/embedding.md)."
                        );
                    }
                }
            }
        }));
    });
}

/// Call Pascal code from a Rust program: a unit built as a library with
/// `pascrust build --lib unit.pas`. A Pascal runtime error (a range check,
/// a nil pointer, a missing file, ...) comes back as `Err` instead of
/// stopping the program:
///
/// ```ignore
/// let found = pascrust_runtime::call(|| tree.search(&mut item))?;
/// ```
pub fn call<T>(pascal: impl FnOnce() -> T) -> Result<T, RuntimeError> {
    quiet_runtime_errors();
    CALL_DEPTH.set(CALL_DEPTH.get() + 1);
    // `catch_unwind` turns a panic into an `Err` holding its payload.
    // (`AssertUnwindSafe` says the caller accepts that what the Pascal code
    // was changing may be left half done, as after a runtime error in TP.)
    let result = panic::catch_unwind(panic::AssertUnwindSafe(pascal)).map_err(to_runtime_error);
    CALL_DEPTH.set(CALL_DEPTH.get() - 1);
    crate::io::flush_stdout();
    result
}

/// For a Rust program done with the Pascal code it called: put the
/// terminal back if `Crt` changed it, and finish printing (`Lst`). A
/// Pascal program does this when it ends.
pub fn shutdown() {
    crate::io::shutdown();
}

/// "Runtime error 201: Range check error ...", as `run` prints it.
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.halted {
            write!(f, "Halt({})", self.code)
        } else if self.message.is_empty() {
            write!(f, "Runtime error {}", self.code)
        } else {
            write!(f, "Runtime error {}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for RuntimeError {}

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
    // Dividing the most negative Integer by -1 doesn't fit either; the
    // CPU's divide instruction trapped on it, and TP reported that as
    // error 200 too.
    let division =
        ["divide by zero", "remainder with a divisor of zero", "divide with overflow", "remainder with overflow"];
    let code = if division.iter().any(|d| message.contains(d)) {
        200
    } else if message.contains("already borrowed") || message.contains("already mutably borrowed") {
        204
    } else {
        255
    };
    RuntimeError { code, message, halted: false }
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

#[cfg(test)]
mod call_tests {
    use super::*;

    #[test]
    fn call_gives_back_a_value_or_the_runtime_error() {
        assert_eq!(call(|| 6 * 7).unwrap(), 42);
        let err = call(|| -> i16 { rte(201, "Range check error") }).unwrap_err();
        assert_eq!(
            (err.code, err.halted, err.to_string().as_str()),
            (201, false, "Runtime error 201: Range check error")
        );
        // After an error, calling again works.
        assert_eq!(call(|| 1).unwrap(), 1);
    }

    #[test]
    fn halt_in_unchecked_io_inside_call_is_still_halt() {
        let err = call(|| {
            io_unchecked(|| halt(1));
            halt(2)
        })
        .unwrap_err();
        assert_eq!((err.code, err.halted), (1, true));
    }

    #[test]
    fn halt_inside_call_comes_back_as_an_err() {
        let err = call(|| halt(3)).unwrap_err();
        assert_eq!((err.code, err.halted, err.to_string().as_str()), (3, true, "Halt(3)"));
    }
}
