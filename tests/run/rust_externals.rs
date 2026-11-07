//! The Rust half of rust_externals.pas.

use ruscal_runtime::prelude::*;

/// A var parameter is a `&mut`.
pub fn double_it(n: &mut i16) {
    *n *= 2;
}

/// A Real is an f64; a Boolean a bool.
pub fn is_close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// Output through the runtime, like Pascal's own Write.
pub fn greet(name: PString) {
    pwriteln!("hello, ", name, " (from Rust)");
}
