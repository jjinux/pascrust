//! Pascal sets: `set of T`, where `T` is an ordinal type with at most 256
//! values whose ordinals are 0..=255 (`Char`, `Byte`, `Boolean`, an
//! enumeration, a subrange like `0..9`).
//!
//! Like Turbo Pascal, a set is a fixed 256-bit bitmap: bit `n` says whether
//! the value with ordinal `n` is in the set. That makes every operation a few
//! machine instructions and the type `Copy`, like TP's 32-byte sets.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, RangeInclusive, Sub, SubAssign};

use crate::ordinal::Ordinal;

/// Pascal `set of T`.
///
/// `PhantomData<T>` is a zero-sized field that only records the element
/// type, so a `PSet<u8>` (set of Char) and a `PSet<Color>` are different
/// types even though both are just 256 bits.
pub struct PSet<T> {
    bits: [u64; 4],
    elem: PhantomData<T>,
}

impl<T> PSet<T> {
    /// The empty set, `[]`.
    pub const fn new() -> PSet<T> {
        PSet { bits: [0; 4], elem: PhantomData }
    }

    /// A set from ranges of ordinals, for set constants:
    /// `const DIGITS: PSet<u8> = PSet::from_ranges(&[(b'0', b'9')]);`.
    /// A `const fn` can run at compile time, so this can initialize a
    /// `const`; it takes plain `u8` ordinals because trait methods (like
    /// `Ordinal::ord`) can't be called there.
    pub const fn from_ranges(ranges: &[(u8, u8)]) -> PSet<T> {
        let mut bits = [0u64; 4];
        // `for` loops aren't allowed in a `const fn` yet; `while` is.
        let mut i = 0;
        while i < ranges.len() {
            let (lo, hi) = ranges[i];
            let mut n = lo as usize;
            while n <= hi as usize {
                bits[n / 64] |= 1 << (n % 64);
                n += 1;
            }
            i += 1;
        }
        PSet { bits, elem: PhantomData }
    }

    /// A set of single elements, given as ordinals, for set constants:
    /// `const VOWELS: PSet<u8> = PSet::of(&[b'a', b'e', b'i', b'o', b'u']);`.
    pub const fn of(elements: &[u8]) -> PSet<T> {
        let mut bits = [0u64; 4];
        let mut i = 0;
        while i < elements.len() {
            let n = elements[i] as usize;
            bits[n / 64] |= 1 << (n % 64);
            i += 1;
        }
        PSet { bits, elem: PhantomData }
    }

    /// The bit for `x`, or `None` if its ordinal is outside 0..=255 (then
    /// it can't be in any set).
    fn bit(x: impl Ordinal) -> Option<(usize, u64)> {
        let n = usize::try_from(x.ord()).ok().filter(|&n| n < 256)?;
        Some((n / 64, 1 << (n % 64)))
    }

    /// Pascal `x in s`. Any ordinal value may be tested: `300 in s` is
    /// simply false.
    pub fn contains(&self, x: impl Ordinal) -> bool {
        Self::bit(x).is_some_and(|(word, mask)| self.bits[word] & mask != 0)
    }

    /// Pascal `Include(s, x)`: `s := s + [x]`.
    pub fn include(&mut self, x: impl Ordinal) {
        if let Some((word, mask)) = Self::bit(x) {
            self.bits[word] |= mask;
        }
    }

    /// Pascal `Exclude(s, x)`: `s := s - [x]`.
    pub fn exclude(&mut self, x: impl Ordinal) {
        if let Some((word, mask)) = Self::bit(x) {
            self.bits[word] &= !mask;
        }
    }

    /// The set with `items` (a value or a range of values) added; used by
    /// the `pset!` macro.
    pub fn with(mut self, items: impl SetItems) -> PSet<T> {
        items.add_to(&mut self.bits);
        self
    }

    /// Pascal `a <= b`: every element of `a` is in `b`.
    pub fn is_subset(&self, other: &PSet<T>) -> bool {
        self.bits.iter().zip(&other.bits).all(|(a, b)| a & !b == 0)
    }

    /// Pascal `a >= b`: every element of `b` is in `a`.
    pub fn is_superset(&self, other: &PSet<T>) -> bool {
        other.is_subset(self)
    }

    /// Combine two sets word by word.
    fn zip_with(self, other: PSet<T>, f: impl Fn(u64, u64) -> u64) -> PSet<T> {
        let mut bits = [0; 4];
        for (i, bit) in bits.iter_mut().enumerate() {
            *bit = f(self.bits[i], other.bits[i]);
        }
        PSet { bits, elem: PhantomData }
    }
}

/// Something a set literal can list: a single value (`x`) or a range of
/// values (`b'a'..=b'z'`).
pub trait SetItems {
    fn add_to(self, bits: &mut [u64; 4]);
}

impl<U: Ordinal> SetItems for U {
    fn add_to(self, bits: &mut [u64; 4]) {
        if let Ok(n @ 0..=255) = usize::try_from(self.ord()) {
            bits[n / 64] |= 1 << (n % 64);
        }
    }
}

// This impl doesn't overlap the one above: `RangeInclusive` isn't an
// `Ordinal`, and since `Ordinal` is this crate's trait, Rust can be sure no
// other crate will make it one.
impl<U: Ordinal> SetItems for RangeInclusive<U> {
    fn add_to(self, bits: &mut [u64; 4]) {
        let (lo, hi) = (self.start().ord().max(0), self.end().ord().min(255));
        for n in lo..=hi {
            let n = n as usize;
            bits[n / 64] |= 1 << (n % 64);
        }
    }
}

/// A Pascal set literal: `pset![b'a'..=b'z', b'_']` is `['a'..'z', '_']`.
/// Where nothing else says what the set holds (`[Mon..Wed, x].contains(d)`),
/// `pset![set of Day; Day::Mon..=Day::Wed, x]` says it, as Pascal would.
/// (`macro_rules!` tries its rules in order; the first only matches when the
/// macro starts with the words `set of`.)
#[macro_export]
macro_rules! pset {
    (set of $t:ty; $($item:expr),* $(,)?) => {
        $crate::set::PSet::<$t>::new()$(.with($item))*
    };
    ($($item:expr),* $(,)?) => {
        $crate::set::PSet::new()$(.with($item))*
    };
}

// Pascal's set operators, as Rust's bitwise ones: `+` (union) is `|`, `*`
// (intersection) is `&`, and `-` (difference) stays `-`.
impl<T> BitOr for PSet<T> {
    type Output = PSet<T>;
    fn bitor(self, other: PSet<T>) -> PSet<T> {
        self.zip_with(other, |a, b| a | b)
    }
}

impl<T> BitAnd for PSet<T> {
    type Output = PSet<T>;
    fn bitand(self, other: PSet<T>) -> PSet<T> {
        self.zip_with(other, |a, b| a & b)
    }
}

impl<T> Sub for PSet<T> {
    type Output = PSet<T>;
    fn sub(self, other: PSet<T>) -> PSet<T> {
        self.zip_with(other, |a, b| a & !b)
    }
}

// And their assigning forms, for `s := s + [x]` (`s |= pset![x]`). A
// `PSet` is `Copy`, so `*self | other` copies the old value out first.
impl<T> BitOrAssign for PSet<T> {
    fn bitor_assign(&mut self, other: PSet<T>) {
        *self = *self | other;
    }
}

impl<T> BitAndAssign for PSet<T> {
    fn bitand_assign(&mut self, other: PSet<T>) {
        *self = *self & other;
    }
}

impl<T> SubAssign for PSet<T> {
    fn sub_assign(&mut self, other: PSet<T>) {
        *self = *self - other;
    }
}

// These are written out rather than derived: `#[derive]` would require `T`
// itself to be `Clone`, `PartialEq`, ..., but only the bits matter.
impl<T> Clone for PSet<T> {
    fn clone(&self) -> PSet<T> {
        *self
    }
}

impl<T> Copy for PSet<T> {}

impl<T> PartialEq for PSet<T> {
    fn eq(&self, other: &PSet<T>) -> bool {
        self.bits == other.bits
    }
}

impl<T> Eq for PSet<T> {}

impl<T> Default for PSet<T> {
    fn default() -> PSet<T> {
        PSet::new()
    }
}

impl<T> PSet<T> {
    /// The set as TP stored it: bit `n % 8` of byte `n / 8` for element `n`,
    /// in as many bytes as its elements need (`bytes`: 1 for a set of
    /// Boolean or of an enum of up to 8 values, 32 for a set of Char).
    /// (`first` is where they start in the 32-byte bitmap: TP keeps a set of
    /// `'a'..'z'` in bytes 12 to 15.)
    pub fn encode_bytes(&self, bytes: usize, first: usize, out: &mut Vec<u8>) {
        let mut all = Vec::new();
        crate::io::FileRecord::encode(self, &mut all);
        out.extend_from_slice(&all[first..first + bytes]);
    }

    pub fn decode_bytes(bytes: usize, first: usize, input: &mut &[u8]) -> PSet<T> {
        let (head, rest) = input.split_at(bytes.min(input.len()));
        *input = rest;
        let mut all = vec![0; first];
        all.extend_from_slice(head);
        all.resize(32, 0);
        crate::io::FileRecord::decode(&mut &all[..])
    }
}

/// A set in a `file of` record: 32 bytes, bit `n % 8` of byte `n / 8` for
/// element `n`, as TP stores a set of 256 elements. (A smaller set in a
/// record field is stored in fewer bytes: see `encode_bytes`.)
impl<T> crate::io::FileRecord for PSet<T> {
    // A set kept in its first `size` bytes (all 32 for 0).
    fn encode_sized(&self, hint: crate::io::Hint, out: &mut Vec<u8>) {
        if hint.size == 0 { self.encode(out) } else { self.encode_bytes(hint.size, hint.first, out) }
    }

    fn decode_sized(hint: crate::io::Hint, input: &mut &[u8]) -> PSet<T> {
        if hint.size == 0 { Self::decode(input) } else { PSet::decode_bytes(hint.size, hint.first, input) }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        for word in self.bits {
            out.extend_from_slice(&word.to_le_bytes());
        }
    }

    fn decode(input: &mut &[u8]) -> PSet<T> {
        let mut set = PSet::new();
        for word in set.bits.iter_mut() {
            // `split_at` cuts off the next 8 bytes; a short file gives zeros.
            let (bytes, rest) = input.split_at(input.len().min(8));
            let mut buf = [0u8; 8];
            buf[..bytes.len()].copy_from_slice(bytes);
            *word = u64::from_le_bytes(buf);
            *input = rest;
        }
        set
    }
}

impl<T> fmt::Debug for PSet<T> {
    /// The ordinals in the set: `{1, 3, 4, 5}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let members = (0..256).filter(|&n| self.bits[n / 64] & (1 << (n % 64)) != 0);
        f.debug_set().entries(members).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_and_operators() {
        let letters: PSet<u8> = pset![b'a'..=b'z', b'_'];
        assert!(letters.contains(b'q') && letters.contains(b'_'));
        assert!(!letters.contains(b'A') && !letters.contains(300));
        let vowels: PSet<u8> = pset![b'a', b'e', b'i', b'o', b'u'];
        assert!(vowels.is_subset(&letters) && !letters.is_subset(&vowels));
        assert_eq!(vowels & letters, vowels);
        assert!(!(letters - vowels).contains(b'e'));
        let mut s = PSet::<u8>::new();
        s.include(3);
        s.include(200);
        assert_eq!(s, pset![3, 200]);
        s.exclude(3);
        assert_eq!(s | pset![1], pset![1, 200]);
        const DIGITS: PSet<u8> = PSet::from_ranges(&[(b'0', b'9')]);
        assert_eq!(DIGITS, pset![b'0'..=b'9']);
        const ODD: PSet<u8> = PSet::of(&[1, 3, 5]);
        assert_eq!(ODD, pset![1, 3, 5]);
        // In a file record: 32 bytes, and back.
        use crate::io::FileRecord;
        let mut bytes = Vec::new();
        letters.encode(&mut bytes);
        assert_eq!(bytes.len(), 32);
        assert_eq!(bytes[b'a' as usize / 8], 0b1111_1110);
        assert_eq!(PSet::<u8>::decode(&mut bytes.as_slice()), letters);
    }
}
