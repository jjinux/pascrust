//! Untyped memory: a variable seen as the bytes Turbo Pascal kept it in.
//!
//! TP let a program treat any variable as raw bytes: an untyped `var`
//! parameter (`procedure Dump(var Data; Size: Word)`), `Move` between
//! variables of different types, `BlockRead` into a record. PascRust can't
//! hand out a variable's address, so instead a variable is turned into its
//! bytes (in TP's layout, the same one typed files use: see `FileRecord`)
//! and back. A routine with an untyped parameter gets a [`Bytes`]: a copy
//! of the bytes, written back into the variable when the routine returns.
//! Nothing else can see the variable in between, so a program can't tell
//! the difference.

use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::io::{FileRecord, FileString, Hint};
use crate::ordinal::Ordinal;

/// A variable as TP's bytes: what an untyped `var` parameter receives
/// (`&mut dyn ByteView`).
pub trait ByteView {
    /// The variable's bytes, as TP laid them out.
    fn to_bytes(&self) -> Vec<u8>;
    /// Set the variable from bytes in that layout (as many as it takes).
    fn set_bytes(&mut self, bytes: &[u8]);
}

/// Anything a typed file can store is a byte view: numbers, records,
/// arrays, enums, sets. (A blanket impl: one `impl` for every type with
/// `FileRecord`.)
impl<T: FileRecord> ByteView for T {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode(&mut out);
        out
    }

    fn set_bytes(&mut self, bytes: &[u8]) {
        *self = T::decode(&mut &bytes[..]);
    }
}

/// A `string[n]` (or an array of them) as bytes: its type says how many
/// bytes it takes, which the `PString` value alone doesn't know.
pub struct StringBytes<'a, T: FileString> {
    value: &'a mut T,
    capacity: usize,
}

/// `Dump(s, ...)` with `s: string[20]`: `&mut string_bytes(&mut s, 20)`.
pub fn string_bytes<T: FileString>(value: &mut T, capacity: usize) -> StringBytes<'_, T> {
    StringBytes { value, capacity }
}

impl<T: FileString> ByteView for StringBytes<'_, T> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.value.encode_string(self.capacity, &mut out);
        out
    }

    fn set_bytes(&mut self, bytes: &[u8]) {
        *self.value = T::decode_string(self.capacity, &mut &bytes[..]);
    }
}

/// A variable whose type gives it a size its Rust type doesn't say: an
/// array of `string[n]` or of small sets, as bytes (see
/// `FileRecord::encode_sized`).
pub struct SizedBytes<'a, T: FileRecord> {
    value: &'a mut T,
    hint: Hint,
}

/// `Dump(names, ...)` with `names: array[1..3] of string[4]`: `&mut
/// sized_bytes(&mut names, 4)`.
pub fn sized_bytes<T: FileRecord>(value: &mut T, size: usize) -> SizedBytes<'_, T> {
    SizedBytes { value, hint: Hint::new(size, 0) }
}

/// For a set kept from byte `first` of its bitmap (`set of 'a'..'z'`).
pub fn sized_bytes_from<T: FileRecord>(value: &mut T, size: usize, first: usize) -> SizedBytes<'_, T> {
    SizedBytes { value, hint: Hint::new(size, first) }
}

impl<T: FileRecord> ByteView for SizedBytes<'_, T> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.value.encode_sized(self.hint, &mut out);
        out
    }

    fn set_bytes(&mut self, bytes: &[u8]) {
        *self.value = T::decode_sized(self.hint, &mut &bytes[..]);
    }
}

/// The same, to read from (`Move`'s source).
pub fn sized_view_bytes<T: FileRecord>(value: &T, size: usize) -> Vec<u8> {
    sized_view_bytes_from(value, size, 0)
}

/// The same, for a set kept from byte `first` of its bitmap.
pub fn sized_view_bytes_from<T: FileRecord>(value: &T, size: usize, first: usize) -> Vec<u8> {
    let mut out = Vec::new();
    value.encode_sized(Hint::new(size, first), &mut out);
    out
}

/// A `string[n]`'s bytes, to read from (`Move`'s source).
pub fn string_view_bytes<T: FileString>(value: &T, capacity: usize) -> Vec<u8> {
    let mut out = Vec::new();
    value.encode_string(capacity, &mut out);
    out
}

/// The bytes of a variable from `offset` on: an untyped argument that
/// starts inside a variable, `BlockWrite(f, s[1], Length(s))` or
/// `Dump(buf[10], 5)`: `&mut bytes_from(&mut buf, 9)`.
pub struct At<'a> {
    view: &'a mut dyn ByteView,
    offset: usize,
}

pub fn bytes_from(view: &mut dyn ByteView, offset: impl Into<i64>) -> At<'_> {
    let offset = checked_offset(offset.into(), view.to_bytes().len());
    At { view, offset }
}

/// The bytes of a variable from `offset` on, to read from (`Move`'s
/// source): `bytes_after(&buf.to_bytes(), 9)`.
pub fn bytes_after(bytes: &[u8], offset: impl Into<i64>) -> &[u8] {
    &bytes[checked_offset(offset.into(), bytes.len())..]
}

/// An argument that starts outside its variable (`buf[0]` of an
/// `array[1..10]`, out of range) is runtime error 201, where TP used
/// whatever was there in memory.
fn checked_offset(offset: i64, len: usize) -> usize {
    match usize::try_from(offset) {
        Ok(at) if at < len => at,
        _ => {
            crate::rte(201, format!("Range check error: an untyped argument at byte {offset} of a {len}-byte variable"))
        }
    }
}

impl ByteView for At<'_> {
    fn to_bytes(&self) -> Vec<u8> {
        self.view.to_bytes().split_off(self.offset)
    }

    fn set_bytes(&mut self, bytes: &[u8]) {
        let mut all = self.view.to_bytes();
        let end = (self.offset + bytes.len()).min(all.len());
        all[self.offset..end].copy_from_slice(&bytes[..end - self.offset]);
        self.view.set_bytes(&all);
    }
}

/// What a routine sees of its untyped parameter: the variable's bytes,
/// which it can index (`TBytes(Data)[i]` is `data[i]`) and pass on. When
/// the routine returns, by any way out (`Exit` too), dropping the `Bytes`
/// writes them back into the variable: the `Drop` trait runs code when a
/// value goes out of scope, as `RefCell`'s `RefMut` does to end a borrow.
pub struct Bytes<'a> {
    view: &'a mut dyn ByteView,
    bytes: Vec<u8>,
    /// The bytes as they came in: unchanged bytes aren't written back, so a
    /// pointer (or procedural) field the routine didn't touch keeps its
    /// value, though as bytes it's only 4 zeros.
    original: Vec<u8>,
}

impl<'a> Bytes<'a> {
    /// `let mut data = Bytes::of(data);` at the start of the routine.
    pub fn of(view: &'a mut dyn ByteView) -> Bytes<'a> {
        let bytes = view.to_bytes();
        Bytes { view, original: bytes.clone(), bytes }
    }

    /// `TRec(Data)`: the bytes read as a `TRec`. Bytes past the end of the
    /// variable read as zeros, where TP read whatever came next in memory.
    pub fn read<T: FileRecord>(&self) -> T {
        let mut bytes = self.bytes.clone();
        bytes.resize(bytes.len().max(T::record_size()), 0);
        T::decode(&mut &bytes[..])
    }

    /// `TName(Data)` for a `string[n]` (or an array of them), which takes
    /// n+1 bytes: its type says how many, which the `PString` doesn't know.
    pub fn read_string<T: FileString>(&self, capacity: usize) -> T {
        let mut bytes = self.bytes.clone();
        bytes.resize(bytes.len().max(capacity + 1), 0);
        T::decode_string(capacity, &mut &bytes[..])
    }

    /// `TName(Data) := s` for a `string[n]`.
    pub fn write_string<T: FileString>(&mut self, value: &T, capacity: usize) {
        let mut new = Vec::new();
        value.encode_string(capacity, &mut new);
        let n = new.len().min(self.bytes.len());
        self.bytes[..n].copy_from_slice(&new[..n]);
    }

    /// `TRec(Data) := r`: the bytes set from a `TRec`.
    pub fn write<T: FileRecord>(&mut self, value: &T) {
        let mut new = Vec::new();
        value.encode(&mut new);
        let n = new.len().min(self.bytes.len());
        self.bytes[..n].copy_from_slice(&new[..n]);
    }
}

impl Drop for Bytes<'_> {
    fn drop(&mut self) {
        if self.bytes != self.original {
            self.view.set_bytes(&self.bytes);
        }
    }
}

impl Deref for Bytes<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}

impl DerefMut for Bytes<'_> {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

/// A byte past the end of the variable is runtime error 201, where TP read
/// or wrote whatever came next in memory.
fn checked(bytes: &[u8], index: i64) -> usize {
    match usize::try_from(index) {
        Ok(i) if i < bytes.len() => i,
        _ => crate::rte(201, format!("Range check error: byte {index} of a {}-byte variable", bytes.len())),
    }
}

/// `data[i]` with any integer type for `i`.
impl<I: Ordinal> Index<I> for Bytes<'_> {
    type Output = u8;
    fn index(&self, index: I) -> &u8 {
        &self.bytes[checked(&self.bytes, index.ord())]
    }
}

impl<I: Ordinal> IndexMut<I> for Bytes<'_> {
    fn index_mut(&mut self, index: I) -> &mut u8 {
        let i = checked(&self.bytes, index.ord());
        &mut self.bytes[i]
    }
}

/// Passing an untyped parameter on to another one.
impl ByteView for Bytes<'_> {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    fn set_bytes(&mut self, bytes: &[u8]) {
        let n = bytes.len().min(self.bytes.len());
        self.bytes[..n].copy_from_slice(&bytes[..n]);
    }
}

/// Pascal `Move(Source, Dest, Count)` between variables of any types: the
/// first `Count` bytes of `Source` replace those of `Dest`. Past the end of
/// either is runtime error 201.
pub fn move_bytes(from: &[u8], dest: &mut dyn ByteView, count: impl Into<i32>) {
    let count = usize::try_from(count.into()).unwrap_or(0);
    let mut to = dest.to_bytes();
    if count > from.len() || count > to.len() {
        crate::rte(201, format!("Range check error: Move of {count} bytes between {} and {}", from.len(), to.len()));
    }
    to[..count].copy_from_slice(&from[..count]);
    dest.set_bytes(&to);
}

/// Pascal `FillChar(X, Count, Value)` on a variable of any type.
pub fn fill_bytes(dest: &mut dyn ByteView, count: impl Into<i32>, value: impl Ordinal) {
    let count = usize::try_from(count.into()).unwrap_or(0);
    let mut bytes = dest.to_bytes();
    if count > bytes.len() {
        crate::rte(201, format!("Range check error: FillChar of {count} bytes into {}", bytes.len()));
    }
    bytes[..count].fill(value.ord() as u8);
    dest.set_bytes(&bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::PArray;
    use crate::string::PString;

    fn sum(data: &mut dyn ByteView) -> u32 {
        let data = Bytes::of(data);
        data.iter().map(|&b| u32::from(b)).sum()
    }

    fn zero_first(data: &mut dyn ByteView) {
        let mut data = Bytes::of(data);
        data[0_i16] = 0;
    }

    #[test]
    fn untyped_parameters_see_tp_bytes_and_write_back() {
        let mut n: i16 = 0x0102;
        assert_eq!(sum(&mut n), 3);
        zero_first(&mut n);
        assert_eq!(n, 0x0100);
        let mut s = PString::from("AB");
        assert_eq!(sum(&mut string_bytes(&mut s, 5)), 2 + 65 + 66);
        let mut a = PArray::<u8, 1, 4>::new([1, 2, 3, 4]);
        zero_first(&mut bytes_from(&mut a, 2));
        assert_eq!(a[3_i16], 0);
    }

    #[test]
    fn move_and_fill_between_types() {
        let mut bytes = PArray::<u8, 1, 2>::new([0, 0]);
        move_bytes(&0x0102_i16.to_bytes(), &mut bytes, 2);
        assert_eq!((bytes[1_i16], bytes[2_i16]), (2, 1));
        let mut s = PString::from("hello");
        fill_bytes(&mut bytes_from(&mut string_bytes(&mut s, 10), 1), 3, b'x');
        assert_eq!(s.to_string(), "xxxlo");
    }
}
