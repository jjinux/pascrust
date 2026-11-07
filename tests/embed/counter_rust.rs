//! The Rust half of counter.pas: an `external` routine that names one of
//! the unit's types as `super::`, which works for a program, a
//! `ruscal build --lib` library and `include_library!` alike.

use super::TTally;

/// Twice an object's total.
pub fn tally_twice(tally: TTally) -> i16 {
    tally.total * 2
}
