//! `PString`: Turbo Pascal's `string` type.
//!
//! A Turbo Pascal string is 256 bytes of memory: byte 0 holds the length and
//! bytes 1..=255 hold the characters. It's a *value* type: `a := b` copies all
//! of it. We use exactly that layout, so `s[0]` (the length byte) and 1-based
//! indexing work just as in Pascal.
//!
//! `string[30]` uses the same type; codegen truncates values assigned to it.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Index, IndexMut};

use crate::ordinal::Ordinal;

// `Clone, Copy`: copying 256 bytes is cheap, and making `PString` `Copy`
// gives it Pascal's value semantics without `.clone()` calls everywhere.
#[derive(Clone, Copy)]
pub struct PString {
    bytes: [u8; 256],
}

impl PString {
    /// The empty string.
    pub const fn new() -> PString {
        PString { bytes: [0; 256] }
    }

    /// Build from bytes, keeping at most 255 of them (as TP would).
    pub fn from_bytes(bytes: &[u8]) -> PString {
        let mut s = PString::new();
        let n = bytes.len().min(255);
        s.bytes[1..=n].copy_from_slice(&bytes[..n]);
        s.bytes[0] = n as u8;
        s
    }

    /// The characters, without the length byte.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[1..=self.bytes[0] as usize]
    }

    fn len_usize(&self) -> usize {
        self.bytes[0] as usize
    }

    /// Pascal `Length(s)`.
    pub fn length(&self) -> i16 {
        i16::from(self.bytes[0])
    }

    /// Keep at most `cap` characters: what assigning to a `string[cap]`
    /// variable does in Turbo Pascal.
    pub fn truncated(mut self, cap: u8) -> PString {
        if self.bytes[0] > cap {
            self.bytes[0] = cap;
        }
        self
    }

    /// Pascal `Copy(s, index, count)`: up to `count` characters starting at
    /// 1-based `index`. Out-of-range arguments give a shorter (or empty)
    /// result rather than an error, as in TP.
    pub fn copy(&self, index: impl Into<i32>, count: impl Into<i32>) -> PString {
        let (index, count) = (index.into(), count.into());
        let len = self.len_usize() as i32;
        if index > len || count <= 0 {
            return PString::new();
        }
        let start = index.max(1);
        let end = (start + count - 1).min(len);
        PString::from_bytes(&self.bytes[start as usize..=end as usize])
    }

    /// Pascal `Delete(s, index, count)`.
    pub fn delete(&mut self, index: impl Into<i32>, count: impl Into<i32>) {
        let (index, count) = (index.into(), count.into());
        let len = self.len_usize() as i32;
        if index < 1 || index > len || count <= 0 {
            return;
        }
        let count = count.min(len - index + 1);
        let mut kept = self.as_bytes()[..(index - 1) as usize].to_vec();
        kept.extend_from_slice(&self.as_bytes()[(index - 1 + count) as usize..]);
        *self = PString::from_bytes(&kept);
    }

    /// Pascal `Insert(source, s, index)`.
    pub fn insert(&mut self, source: impl Into<PString>, index: impl Into<i32>) {
        let source = source.into();
        let index = (index.into().max(1) as usize).min(self.len_usize() + 1);
        let mut bytes = self.as_bytes()[..index - 1].to_vec();
        bytes.extend_from_slice(source.as_bytes());
        bytes.extend_from_slice(&self.as_bytes()[index - 1..]);
        *self = PString::from_bytes(&bytes);
    }
}

/// Pascal `Pos(substr, s)`: the 1-based position of the first occurrence, or
/// 0 if there is none.
pub fn pos(substr: impl Into<PString>, s: impl Into<PString>) -> i16 {
    let (substr, s) = (substr.into(), s.into());
    let (needle, hay) = (substr.as_bytes(), s.as_bytes());
    if needle.is_empty() {
        return 0;
    }
    // `windows(n)` yields every run of `n` consecutive bytes.
    hay.windows(needle.len()).position(|w| w == needle).map_or(0, |i| i as i16 + 1)
}

/// Convert anything string-like into a `PString`: string literals, byte
/// string literals, characters and `PString`s themselves.
pub fn pstr(value: impl Into<PString>) -> PString {
    value.into()
}

impl Default for PString {
    fn default() -> PString {
        PString::new()
    }
}

// `From` impls give us `Into` for free, which is what `pstr` and the
// operators below accept.
impl From<&str> for PString {
    fn from(s: &str) -> PString {
        PString::from_bytes(s.as_bytes())
    }
}

impl From<&[u8]> for PString {
    fn from(s: &[u8]) -> PString {
        PString::from_bytes(s)
    }
}

impl<const N: usize> From<&[u8; N]> for PString {
    fn from(s: &[u8; N]) -> PString {
        PString::from_bytes(s)
    }
}

/// A `Char` converts to a one-character string, as in Pascal.
impl From<u8> for PString {
    fn from(c: u8) -> PString {
        PString::from_bytes(&[c])
    }
}

/// Pascal `+` on strings concatenates (and truncates at 255 characters).
impl<T: Into<PString>> Add<T> for PString {
    type Output = PString;
    fn add(self, rhs: T) -> PString {
        let rhs = rhs.into();
        let mut bytes = self.as_bytes().to_vec();
        bytes.extend_from_slice(rhs.as_bytes());
        PString::from_bytes(&bytes)
    }
}

/// Strings are indexed from 1; `s[0]` is the length byte.
impl<I: Ordinal> Index<I> for PString {
    type Output = u8;
    fn index(&self, i: I) -> &u8 {
        &self.bytes[string_index(i)]
    }
}

impl<I: Ordinal> IndexMut<I> for PString {
    fn index_mut(&mut self, i: I) -> &mut u8 {
        &mut self.bytes[string_index(i)]
    }
}

fn string_index<I: Ordinal>(i: I) -> usize {
    let i = i.ord();
    if !(0..=255).contains(&i) {
        crate::rte(201, format!("Range check error: string index {i} is not in 0..255"));
    }
    i as usize
}

// Equality and ordering compare only the characters, not the unused bytes
// after them, so these can't be derived.
impl PartialEq for PString {
    fn eq(&self, other: &PString) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for PString {}

impl PartialEq<&str> for PString {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<u8> for PString {
    fn eq(&self, other: &u8) -> bool {
        self.as_bytes() == [*other]
    }
}

/// Byte-by-byte comparison, shorter first on a tie: Turbo Pascal's rule.
impl Ord for PString {
    fn cmp(&self, other: &PString) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl PartialOrd for PString {
    fn partial_cmp(&self, other: &PString) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<&str> for PString {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        Some(self.as_bytes().cmp(other.as_bytes()))
    }
}

impl PartialOrd<u8> for PString {
    fn partial_cmp(&self, other: &u8) -> Option<Ordering> {
        Some(self.as_bytes().cmp(&[*other][..]))
    }
}

impl fmt::Debug for PString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", String::from_utf8_lossy(self.as_bytes()))
    }
}

impl fmt::Display for PString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let s = pstr("hello");
        assert_eq!(s.length(), 5);
        assert_eq!(s[1], b'h');
        assert_eq!(s[0], 5);
        assert_eq!(s + " world", "hello world");
        assert_eq!(s + b'!', "hello!");
        assert!(pstr("abc") < pstr("abd"));
        assert!(pstr("ab") < "abc");
    }

    #[test]
    fn copy_delete_insert_pos() {
        let s = pstr("Turbo Pascal");
        assert_eq!(s.copy(7, 6), "Pascal");
        assert_eq!(s.copy(7, 100), "Pascal");
        assert_eq!(s.copy(50, 2), "");
        let mut t = s;
        t.delete(1, 6);
        assert_eq!(t, "Pascal");
        t.insert("Free ", 1);
        assert_eq!(t, "Free Pascal");
        assert_eq!(pos("Pas", s), 7);
        assert_eq!(pos("xyz", s), 0);
    }

    #[test]
    fn truncation() {
        assert_eq!(pstr("abcdef").truncated(3), "abc");
        let long = vec![b'x'; 300];
        assert_eq!(PString::from_bytes(&long).length(), 255);
    }

    #[test]
    fn setting_the_length_byte() {
        let mut s = pstr("hello");
        s[0] = 2;
        assert_eq!(s, "he");
    }
}
