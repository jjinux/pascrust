//! `PArray`: Pascal arrays, which can start at any index.
//!
//! In Pascal you write `array[1..10] of Integer`, `array['a'..'z'] of
//! Boolean` or `array[Red..Blue] of Real`. Rust arrays always start at 0.
//! `PArray<T, LO, N>` wraps a Rust array of `N` elements and subtracts the
//! lower bound `LO` on every access, checking the range as it goes.
//!
//! `LO` and `N` are *const generics*: compile-time constants that are part of
//! the type, like the `3` in `[i16; 3]`. So `PArray<i16, 1, 10>` and
//! `PArray<i16, 0, 10>` are different types, just as `array[1..10]` and
//! `array[0..9]` are different Pascal types. (Ideally the parameters would be
//! `LO` and `HI`, but computing `HI - LO + 1` from generic parameters isn't
//! possible in stable Rust yet, so the transpiler passes the length.)

use std::fmt;
use std::ops::{Index, IndexMut};

use crate::ordinal::Ordinal;

#[derive(Clone, Copy, PartialEq)]
pub struct PArray<T, const LO: i64, const N: usize>([T; N]);

impl<T, const LO: i64, const N: usize> PArray<T, LO, N> {
    /// An array with the given elements, for typed constants like
    /// `const Days: array[1..3] of Integer = (31, 28, 31);`.
    pub const fn new(elements: [T; N]) -> Self {
        PArray(elements)
    }

    /// Turn a Pascal index into a Rust index, or raise runtime error 201,
    /// Turbo Pascal's "Range check error".
    fn offset(index: i64) -> usize {
        let hi = LO + N as i64 - 1;
        if index < LO || index > hi {
            crate::rte(201, format!("Range check error: index {index} is not in {LO}..{hi}"));
        }
        (index - LO) as usize
    }

    /// Iterate over the elements in index order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }
}

/// Pascal leaves arrays uninitialized; we zero them by starting every
/// element at its default. The standard library only implements `Default`
/// for arrays of up to 32 elements, so build it with `std::array::from_fn`,
/// which calls the closure once per element.
impl<T: Default, const LO: i64, const N: usize> Default for PArray<T, LO, N> {
    fn default() -> Self {
        PArray(std::array::from_fn(|_| T::default()))
    }
}

/// Indexing with any ordinal: an integer of any width, a `Char` (`u8`) or an
/// enum value. Implementing `Index` is what makes `a[i]` syntax work.
impl<T, I: Ordinal, const LO: i64, const N: usize> Index<I> for PArray<T, LO, N> {
    type Output = T;
    fn index(&self, index: I) -> &T {
        &self.0[Self::offset(index.ord())]
    }
}

impl<T, I: Ordinal, const LO: i64, const N: usize> IndexMut<I> for PArray<T, LO, N> {
    fn index_mut(&mut self, index: I) -> &mut T {
        &mut self.0[Self::offset(index.ord())]
    }
}

impl<T: fmt::Debug, const LO: i64, const N: usize> fmt::Debug for PArray<T, LO, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.0.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_are_shifted() {
        let mut a: PArray<i16, 1, 3> = PArray::new([10, 20, 30]);
        assert_eq!(a[1], 10);
        assert_eq!(a[3i16], 30);
        a[2u8] = 99;
        assert_eq!(a[2], 99);
    }

    #[test]
    fn char_indexes() {
        let mut counts: PArray<i32, 97, 26> = PArray::default();
        counts[b'c'] += 1;
        assert_eq!(counts[b'c'], 1);
    }

    #[test]
    fn big_arrays_have_defaults() {
        let grid: PArray<PArray<bool, 0, 22>, 0, 78> = PArray::default();
        assert!(!grid[77][21]);
    }

    #[test]
    #[should_panic]
    fn out_of_range_is_a_runtime_error() {
        let a: PArray<i16, 1, 3> = PArray::default();
        let _ = a[4];
    }
}
