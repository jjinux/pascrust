//! `PChar` and the `Strings` unit: C-style strings, a run of characters
//! ended by a `#0`, with no length byte and no 255-character limit.
//!
//! Turbo Pascal's `PChar` was a `^Char`: an address, with arithmetic on it
//! (`P + 1`, `P - Q`) and indexing (`P[5]`). Here a `PChar` is a shared byte
//! buffer and an offset into it. The buffer is the same kind `GetMem` gives
//! a `Pointer` (`Rc<RefCell<Vec<u8>>>`), so `PChar(p)` can share it.
//! Reading or writing past the end of the buffer is runtime error 201,
//! where TP read or overwrote whatever came next in memory.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use crate::ptr::AnyPtr;
use crate::string::PString;

#[derive(Clone, Default)]
pub struct PChar {
    buffer: Option<Rc<RefCell<Vec<u8>>>>,
    /// Where the string starts in `buffer`. Signed, because `P - 1` is
    /// allowed as long as nothing reads there.
    offset: i64,
}

impl PChar {
    pub const fn nil() -> PChar {
        PChar { buffer: None, offset: 0 }
    }

    pub fn is_nil(&self) -> bool {
        self.buffer.is_none()
    }

    fn new(bytes: Vec<u8>) -> PChar {
        PChar { buffer: Some(Rc::new(RefCell::new(bytes))), offset: 0 }
    }

    /// A string literal used as a `PChar` (`P := 'Hello'`): a `#0`-ended
    /// copy of it, as TP made one in the program's data.
    pub fn literal(text: impl Into<PString>) -> PChar {
        let mut bytes = text.into().as_bytes().to_vec();
        bytes.push(0);
        PChar::new(bytes)
    }

    /// A zero-based `array[0..N] of Char` used as a `PChar`: a copy of its
    /// characters, which `copy_to` writes back (see architecture.md for why
    /// it's a copy).
    pub fn copy_of(chars: &[u8]) -> PChar {
        PChar::new(chars.to_vec())
    }

    /// Write the characters back into the array they were copied from.
    pub fn copy_to(&self, chars: &mut [u8]) {
        let buffer = self.buffer().borrow();
        let n = chars.len().min(buffer.len());
        chars[..n].copy_from_slice(&buffer[..n]);
    }

    /// `GetMem(P, Size)`: `Size` zero bytes.
    pub fn alloc(size: impl Into<i32>) -> PChar {
        PChar::new(vec![0; usize::try_from(size.into()).unwrap_or(0)])
    }

    /// `FreeMem(P, Size)` or `StrDispose(P)`: the bytes are gone, and using
    /// them again is runtime error 201.
    pub fn free(&mut self) {
        if let Some(buffer) = &self.buffer {
            buffer.borrow_mut().clear();
        }
    }

    /// `PChar(p)` for a `Pointer` from `GetMem`: its bytes as characters.
    pub fn from_any(pointer: &AnyPtr) -> PChar {
        match pointer {
            AnyPtr::Nil => PChar::nil(),
            AnyPtr::Bytes(bytes) => PChar { buffer: Some(bytes.clone()), offset: 0 },
            AnyPtr::Typed(_) | AnyPtr::Code(_) => {
                crate::rte(204, "Invalid pointer operation: a Pointer to something else used as a PChar")
            }
        }
    }

    /// `Pointer(P)`: the bytes as untyped memory. Only a `PChar` to the start
    /// of its bytes: a `Pointer` here has no offset.
    pub fn to_any(&self) -> AnyPtr {
        match &self.buffer {
            None => AnyPtr::Nil,
            Some(bytes) if self.offset == 0 => AnyPtr::Bytes(bytes.clone()),
            Some(_) => {
                crate::rte(204, "Invalid pointer operation: a PChar into the middle of a string used as a Pointer")
            }
        }
    }

    /// `P + N`.
    pub fn plus(&self, n: impl Into<i32>) -> PChar {
        PChar { buffer: self.buffer.clone(), offset: self.offset + i64::from(n.into()) }
    }

    /// `P - N`.
    pub fn minus(&self, n: impl Into<i32>) -> PChar {
        PChar { buffer: self.buffer.clone(), offset: self.offset - i64::from(n.into()) }
    }

    /// `P - Q`: how many characters `Q` is before `P`, as a `Word` (TP
    /// subtracted the addresses' offset parts).
    pub fn diff(&self, other: &PChar) -> u16 {
        (self.offset - other.offset) as u16
    }

    fn buffer(&self) -> &Rc<RefCell<Vec<u8>>> {
        match &self.buffer {
            Some(buffer) => buffer,
            None => crate::rte(204, "Invalid pointer operation: a nil PChar"),
        }
    }

    /// Where `P[i]` is in the buffer, if it's inside it.
    fn index(&self, i: i64, len: usize) -> usize {
        match usize::try_from(self.offset + i) {
            Ok(at) if at < len => at,
            _ => crate::rte(201, format!("Range check error: a PChar past the end of its {len} characters")),
        }
    }

    /// `P[i]` (and `P^`, which is `P[0]`).
    pub fn get(&self, i: impl Into<i32>) -> u8 {
        let buffer = self.buffer().borrow();
        buffer[self.index(i64::from(i.into()), buffer.len())]
    }

    /// `P[i] := C`.
    pub fn set(&self, i: impl Into<i32>, c: u8) {
        let mut buffer = self.buffer().borrow_mut();
        let at = self.index(i64::from(i.into()), buffer.len());
        buffer[at] = c;
    }

    /// The characters before the `#0`. (A `PChar` with no `#0` before the
    /// end of its buffer is runtime error 201.)
    pub fn chars(&self) -> Vec<u8> {
        let buffer = self.buffer().borrow();
        let start = self.index(0, buffer.len().max(1));
        match buffer.get(start..).and_then(|rest| rest.iter().position(|&c| c == 0)) {
            Some(n) => buffer[start..start + n].to_vec(),
            None => crate::rte(201, "Range check error: a PChar with no #0 at its end"),
        }
    }

    /// Copy `bytes` to `P[0]`, `P[1]`, ...
    fn put(&self, bytes: &[u8]) {
        for (i, &c) in bytes.iter().enumerate() {
            self.set(i as i32, c);
        }
    }
}

/// `Inc(P)` and `Inc(P, N)`: `p += 1;`. (`AddAssign` is the trait behind
/// `+=`; this impl accepts any integer type for `N`.)
impl<N: Into<i32>> std::ops::AddAssign<N> for PChar {
    fn add_assign(&mut self, n: N) {
        self.offset += i64::from(n.into());
    }
}

/// `Dec(P)`: `p -= 1;`.
impl<N: Into<i32>> std::ops::SubAssign<N> for PChar {
    fn sub_assign(&mut self, n: N) {
        self.offset -= i64::from(n.into());
    }
}

/// Two `PChar`s are equal when they point at the same character.
impl PartialEq for PChar {
    fn eq(&self, other: &PChar) -> bool {
        match (&self.buffer, &other.buffer) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b) && self.offset == other.offset,
            _ => false,
        }
    }
}

/// `P < Q` compares where they point, which only means something inside
/// one string (TP compared the offsets alone).
impl PartialOrd for PChar {
    fn partial_cmp(&self, other: &PChar) -> Option<Ordering> {
        match (&self.buffer, &other.buffer) {
            (Some(a), Some(b)) if Rc::ptr_eq(a, b) => Some(self.offset.cmp(&other.offset)),
            _ if self == other => Some(Ordering::Equal),
            _ => None,
        }
    }
}

impl fmt::Debug for PChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.buffer {
            None => write!(f, "nil"),
            Some(_) => write!(f, "PChar(+{})", self.offset),
        }
    }
}

/// `Write(P)`: the characters up to the `#0`; nothing for `nil`.
impl crate::io::PasWrite for PChar {
    fn pas_write(&self, out: &mut Vec<u8>) {
        if !self.is_nil() {
            out.extend(self.chars());
        }
    }
}

/// As for other pointers, a stored `PChar` is 4 zero bytes and reads back
/// as nil.
impl crate::io::FileRecord for PChar {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[0; 4]);
    }
    fn decode(input: &mut &[u8]) -> Self {
        *input = &input[4.min(input.len())..];
        PChar::nil()
    }
}

// ----- The Strings unit -----------------------------------------------------

/// `StrLen(Str)`: the number of characters before the `#0`.
pub fn str_len(s: &PChar) -> u16 {
    s.chars().len() as u16
}

/// `StrEnd(Str)`: a pointer to the `#0` that ends `Str`.
pub fn str_end(s: &PChar) -> PChar {
    s.plus(s.chars().len() as i32)
}

/// `StrMove(Dest, Source, Count)`: copy exactly `Count` characters, which
/// may overlap; returns `Dest`.
pub fn str_move(dest: &PChar, source: &PChar, count: impl Into<i32>) -> PChar {
    let bytes: Vec<u8> = (0..count.into()).map(|i| source.get(i)).collect();
    dest.put(&bytes);
    dest.clone()
}

/// `StrCopy(Dest, Source)`: returns `Dest`.
pub fn str_copy(dest: &PChar, source: &PChar) -> PChar {
    let mut bytes = source.chars();
    bytes.push(0);
    dest.put(&bytes);
    dest.clone()
}

/// `StrECopy(Dest, Source)`: like `StrCopy`, but returns a pointer to the
/// `#0` at the end of `Dest`, for chaining copies.
pub fn str_e_copy(dest: &PChar, source: &PChar) -> PChar {
    let n = source.chars().len();
    str_copy(dest, source);
    dest.plus(n as i32)
}

/// `StrLCopy(Dest, Source, MaxLen)`: at most `MaxLen` characters.
pub fn str_l_copy(dest: &PChar, source: &PChar, max_len: impl Into<i32>) -> PChar {
    let mut bytes = source.chars();
    bytes.truncate(usize::try_from(max_len.into()).unwrap_or(0));
    bytes.push(0);
    dest.put(&bytes);
    dest.clone()
}

/// `StrPCopy(Dest, Source)`: a Pascal string into a `PChar`.
pub fn str_p_copy(dest: &PChar, source: impl Into<PString>) -> PChar {
    let mut bytes = source.into().as_bytes().to_vec();
    bytes.push(0);
    dest.put(&bytes);
    dest.clone()
}

/// `StrCat(Dest, Source)`: append; returns `Dest`.
pub fn str_cat(dest: &PChar, source: &PChar) -> PChar {
    str_copy(&str_end(dest), source);
    dest.clone()
}

/// `StrLCat(Dest, Source, MaxLen)`: append, keeping `Dest` to at most
/// `MaxLen` characters.
pub fn str_l_cat(dest: &PChar, source: &PChar, max_len: impl Into<i32>) -> PChar {
    let room = max_len.into() - dest.chars().len() as i32;
    str_l_copy(&str_end(dest), source, room.max(0));
    dest.clone()
}

/// The comparisons: the difference of the first characters that differ, or
/// 0, looking at no more than `max_len` characters (`None` for all).
fn compare(a: &PChar, b: &PChar, max_len: Option<i32>, fold: fn(u8) -> u8) -> i16 {
    let (a, b) = (a.chars(), b.chars());
    let limit = max_len.map_or(usize::MAX, |n| usize::try_from(n).unwrap_or(0));
    // Compare the `#0`s too, so a prefix is less than the longer string.
    let a = a.iter().copied().chain([0]);
    let b = b.iter().copied().chain([0]);
    for (x, y) in a.zip(b).take(limit) {
        let (x, y) = (fold(x), fold(y));
        if x != y || x == 0 {
            return i16::from(x) - i16::from(y);
        }
    }
    0
}

/// `StrComp(Str1, Str2)`: below 0 if `Str1` comes first, 0 if they're equal,
/// above 0 if `Str2` comes first.
pub fn str_comp(a: &PChar, b: &PChar) -> i16 {
    compare(a, b, None, |c| c)
}

/// `StrIComp(Str1, Str2)`: `StrComp`, ignoring case.
pub fn str_i_comp(a: &PChar, b: &PChar) -> i16 {
    compare(a, b, None, |c| c.to_ascii_uppercase())
}

/// `StrLComp(Str1, Str2, MaxLen)`: the first `MaxLen` characters.
pub fn str_l_comp(a: &PChar, b: &PChar, max_len: impl Into<i32>) -> i16 {
    compare(a, b, Some(max_len.into()), |c| c)
}

/// `StrLIComp(Str1, Str2, MaxLen)`.
pub fn str_l_i_comp(a: &PChar, b: &PChar, max_len: impl Into<i32>) -> i16 {
    compare(a, b, Some(max_len.into()), |c| c.to_ascii_uppercase())
}

/// `StrScan(Str, Chr)`: the first `Chr` in `Str`, or nil. (`#0` finds the
/// end.)
pub fn str_scan(s: &PChar, c: u8) -> PChar {
    let chars = s.chars();
    match chars.iter().chain([&0]).position(|&x| x == c) {
        Some(i) => s.plus(i as i32),
        None => PChar::nil(),
    }
}

/// `StrRScan(Str, Chr)`: the last `Chr` in `Str`, or nil.
pub fn str_r_scan(s: &PChar, c: u8) -> PChar {
    // With the `#0` at the end, so `StrRScan(P, #0)` finds it.
    let mut chars = s.chars();
    chars.push(0);
    match chars.iter().rposition(|&x| x == c) {
        Some(i) => s.plus(i as i32),
        None => PChar::nil(),
    }
}

/// `StrPos(Str1, Str2)`: the first place `Str2` occurs in `Str1`, or nil.
pub fn str_pos(s: &PChar, find: &PChar) -> PChar {
    let (chars, find) = (s.chars(), find.chars());
    if find.is_empty() {
        return PChar::nil();
    }
    match chars.windows(find.len()).position(|w| w == find) {
        Some(i) => s.plus(i as i32),
        None => PChar::nil(),
    }
}

/// `StrUpper(Str)`: in place; returns `Str`.
pub fn str_upper(s: &PChar) -> PChar {
    s.put(&s.chars().to_ascii_uppercase());
    s.clone()
}

/// `StrLower(Str)`: in place; returns `Str`.
pub fn str_lower(s: &PChar) -> PChar {
    s.put(&s.chars().to_ascii_lowercase());
    s.clone()
}

/// `StrPas(Str)`: a Pascal string (the first 255 characters).
pub fn str_pas(s: &PChar) -> PString {
    if s.is_nil() {
        return PString::default();
    }
    let chars = s.chars();
    PString::from(&chars[..chars.len().min(255)])
}

/// `StrNew(Str)`: a copy on the heap, or nil for nil or an empty string.
pub fn str_new(s: &PChar) -> PChar {
    if s.is_nil() || s.chars().is_empty() {
        return PChar::nil();
    }
    let mut bytes = s.chars();
    bytes.push(0);
    PChar::new(bytes)
}

/// `StrDispose(Str)`: free a string from `StrNew`. (nil is fine.)
pub fn str_dispose(s: &PChar) {
    s.clone().free();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_and_pointers() {
        let buf = PChar::alloc(20);
        str_copy(&buf, &PChar::literal("hello"));
        str_cat(&buf, &PChar::literal(", world"));
        assert_eq!(str_pas(&buf).to_string(), "hello, world");
        assert_eq!(str_len(&buf), 12);
        let comma = str_scan(&buf, b',');
        assert_eq!(comma.diff(&buf), 5);
        assert!(comma > buf);
        assert_eq!(str_pas(&comma.plus(2)).to_string(), "world");
        assert_eq!(str_pas(&str_pos(&buf, &PChar::literal("wor"))).to_string(), "world");
        assert!(str_scan(&buf, b'z').is_nil());
        assert_eq!(str_end(&buf).get(0), 0);
        let end = str_e_copy(&buf, &PChar::literal("ab"));
        str_copy(&end, &PChar::literal("cd"));
        assert_eq!(str_pas(&str_upper(&buf)).to_string(), "ABCD");
    }

    #[test]
    fn comparisons() {
        let (a, b) = (PChar::literal("abc"), PChar::literal("abd"));
        assert!(str_comp(&a, &b) < 0);
        assert!(str_comp(&b, &a) > 0);
        assert_eq!(str_l_comp(&a, &b, 2), 0);
        assert_eq!(str_i_comp(&a, &PChar::literal("ABC")), 0);
        assert!(str_comp(&PChar::literal("ab"), &a) < 0);
    }

    #[test]
    fn arrays_copy_in_and_back() {
        let mut array = [0u8; 8];
        let p = PChar::copy_of(&array);
        str_p_copy(&p, "hi");
        p.copy_to(&mut array);
        assert_eq!(&array[..3], b"hi\0");
    }

    #[test]
    #[should_panic]
    fn past_the_end_is_an_error() {
        str_copy(&PChar::alloc(3), &PChar::literal("hello"));
    }
}
