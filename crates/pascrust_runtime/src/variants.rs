//! A variant part without a tag field: the bytes Turbo Pascal kept there.
//!
//! `case Integer of 0: (L: LongInt); 1: (Lo, Hi: Word)` lets a program write
//! `L` and read `Lo` and `Hi`: the variants overlay the same memory. Only
//! keeping that memory gets this right, so a record with such a part holds a
//! [`VariantPart`], the bytes in TP's layout, and each field becomes small
//! methods that read or write its slice (`p.lo()`, `p.set_lo(5)`,
//! `p.lo_mut()`), through the same encoding typed files use (`FileRecord`).

use std::ops::{Deref, DerefMut};

use crate::io::{FileRecord, FileString};

/// The `N` bytes a tagless variant part takes in TP (its largest variant).
#[derive(Clone, Copy, Debug)]
pub struct VariantPart<const N: usize>([u8; N]);

// Rust only derives `Default` for arrays of up to 32 elements, so this one
// is written out: all zero bytes, as a fresh record in TP's data segment.
impl<const N: usize> Default for VariantPart<N> {
    fn default() -> Self {
        VariantPart([0; N])
    }
}

impl<const N: usize> VariantPart<N> {
    /// The field at `offset`, read from the bytes.
    pub fn get<T: FileRecord>(&self, offset: usize) -> T {
        T::decode(&mut &self.0[offset..])
    }

    /// Set the field at `offset`: its bytes change, and so does every field
    /// of another variant that overlaps them.
    pub fn set<T: FileRecord>(&mut self, offset: usize, value: T) {
        let mut bytes = Vec::new();
        value.encode(&mut bytes);
        put(&mut self.0[offset..], &bytes);
    }

    /// The field at `offset`, to change in place (`Inc(p.Hi)`, `p.B[4] :=
    /// 0`): see [`VariantField`].
    pub fn field<T: FileRecord>(&mut self, offset: usize) -> VariantField<'_, T> {
        let value = self.get(offset);
        VariantField { bytes: &mut self.0[offset..], value, encode: |value, out| value.encode(out) }
    }

    /// A `string[cap]` field (or an array of them): its type says how many
    /// bytes it takes, which the `PString` alone doesn't know.
    pub fn get_string<T: FileString>(&self, offset: usize, cap: usize) -> T {
        T::decode_string(cap, &mut &self.0[offset..])
    }

    pub fn set_string<T: FileString>(&mut self, offset: usize, cap: usize, value: T) {
        let mut bytes = Vec::new();
        value.encode_string(cap, &mut bytes);
        put(&mut self.0[offset..], &bytes);
    }

    pub fn string_field<T: FileString>(&mut self, offset: usize, cap: usize) -> StringField<'_, T> {
        let value = self.get_string(offset, cap);
        StringField { bytes: &mut self.0[offset..], value, cap }
    }
}

/// Copy a field's bytes in, as many as fit (sema sizes the part so they all
/// do).
fn put(slot: &mut [u8], bytes: &[u8]) {
    let n = bytes.len().min(slot.len());
    slot[..n].copy_from_slice(&bytes[..n]);
}

/// A variant field being changed in place: the value read from the bytes,
/// which it derefs to, and written back when the guard is dropped at the
/// end of the statement (`Drop` runs then), as the `Bytes` of an untyped
/// parameter is.
pub struct VariantField<'a, T> {
    bytes: &'a mut [u8],
    value: T,
    encode: fn(&T, &mut Vec<u8>),
}

impl<T> Deref for VariantField<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for VariantField<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> Drop for VariantField<'_, T> {
    fn drop(&mut self) {
        let mut bytes = Vec::new();
        (self.encode)(&self.value, &mut bytes);
        put(self.bytes, &bytes);
    }
}

/// [`VariantField`] for a `string[cap]` field.
pub struct StringField<'a, T: FileString> {
    bytes: &'a mut [u8],
    value: T,
    cap: usize,
}

impl<T: FileString> Deref for StringField<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: FileString> DerefMut for StringField<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: FileString> Drop for StringField<'_, T> {
    fn drop(&mut self) {
        let mut bytes = Vec::new();
        self.value.encode_string(self.cap, &mut bytes);
        put(self.bytes, &bytes);
    }
}

/// In a file (or seen as bytes), a variant part is its bytes as they are.
impl<const N: usize> FileRecord for VariantPart<N> {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }

    fn decode(input: &mut &[u8]) -> Self {
        let mut bytes = [0; N];
        let n = N.min(input.len());
        bytes[..n].copy_from_slice(&input[..n]);
        *input = &input[n..];
        VariantPart(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::PString;

    #[test]
    fn fields_overlay_the_same_bytes() {
        let mut part = VariantPart::<4>::default();
        part.set(0, 0x1234_5678_i32);
        assert_eq!((part.get::<u16>(0), part.get::<u16>(2)), (0x5678, 0x1234));
        *part.field::<u16>(2) += 1;
        assert_eq!(part.get::<i32>(0), 0x1235_5678);
        part.set_string(0, 3, PString::from("ab"));
        assert_eq!(part.get::<u8>(0), 2);
        part.string_field::<PString>(0, 3).insert("c", 3);
        assert_eq!(part.get_string::<PString>(0, 3).to_string(), "abc");
    }
}
