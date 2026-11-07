//! The Rust half of rustlink.pas: its `external` routines.

use ruscal_runtime::prelude::*;

/// The greatest common divisor, by Euclid's algorithm.
pub fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

/// The string backwards.
pub fn reversed(s: PString) -> PString {
    let mut bytes = s.as_bytes().to_vec();
    bytes.reverse();
    PString::from(bytes.as_slice())
}
