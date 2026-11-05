//! The Rust half of calls_rust.pas: routines the Pascal program declares
//! `external`, linked with `{$L calls_rust.rs}`. Each takes PascRust's
//! usual types (Integer is `i16`, a string `PString`, a `var` parameter
//! `&mut`); `pascrust emit calls_rust.pas` shows the signatures.

use super::{TLess, TNumbers};
use pascrust_runtime::prelude::*;

/// An environment variable, or '' if it isn't set.
pub fn env_var(name: PString) -> PString {
    let value = std::env::var(name.to_string()).unwrap_or_default();
    PString::from(value.as_str())
}

/// How many words a line has (Rust's `split_whitespace`).
pub fn word_count(line: PString) -> i16 {
    line.to_string().split_whitespace().count() as i16
}

/// True if `n` is prime.
pub fn is_prime(n: i32) -> bool {
    n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| n % d != 0)
}

/// Sort the numbers with Rust's own sort, asking Pascal's `Less` which of
/// two comes first: Rust calling back into Pascal.
pub fn sort_numbers(numbers: &mut TNumbers, less: TLess) {
    let less = less.get();
    let mut values: Vec<i16> = (1..=8).map(|i| numbers[i]).collect();
    values.sort_by(|a, b| {
        if less(*a, *b) {
            std::cmp::Ordering::Less
        } else if less(*b, *a) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    for (i, value) in (1..=8).zip(values) {
        numbers[i] = value;
    }
}
