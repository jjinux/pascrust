//! Pascal files: `Text` (lines of characters) and `file of T` (a sequence
//! of fixed-size records with random access).
//!
//! A Pascal file variable starts out unconnected. `Assign(f, name)` gives it
//! a file name, `Reset` opens it for reading, `Rewrite` creates it (empty)
//! for writing, and `Close` closes it. DOS file names are translated with
//! [`crate::system::dos_path`].
//!
//! Errors are Turbo Pascal runtime errors: 2 "File not found", 102 "File
//! not assigned", 103 "File not open", 104 "File not open for input", 105
//! "File not open for output", 100 "Disk read error" (reading past the end).

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::marker::PhantomData;

use super::read::{LineReader, PasRead, Source};
use crate::string::PString;
use crate::system::dos_path;

fn open_error(e: std::io::Error, name: &PString) -> ! {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => 2,
        std::io::ErrorKind::PermissionDenied => 5,
        _ => 100,
    };
    crate::rte(code, format!("{e}: {name}"))
}

/// `Erase`: DOS refused to delete a read-only file (access denied); Unix
/// would delete it, since that's up to the directory.
fn remove_file(path: &std::path::Path) -> std::io::Result<()> {
    if std::fs::metadata(path)?.permissions().readonly() {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    std::fs::remove_file(path)
}

/// A file variable of either kind, for the `Dos` routines that take any
/// file (`GetFTime(var F; ...)`): its assigned name, if it has one.
pub trait AnyFile {
    fn assigned_name(&self) -> Option<PString>;

    /// Write out anything buffered, so that it doesn't land after a
    /// `SetFTime` and change the file's time again. (A default method:
    /// typed files write straight through and need nothing.)
    fn flush_buffer(&mut self) {}
}

impl AnyFile for PText {
    fn assigned_name(&self) -> Option<PString> {
        self.name
    }

    fn flush_buffer(&mut self) {
        if let TextMode::Writing(writer) = &mut self.mode {
            let _ = writer.flush();
        }
    }
}

impl<T> AnyFile for PFile<T> {
    fn assigned_name(&self) -> Option<PString> {
        self.name
    }
}

// ----- Text files -----------------------------------------------------------

/// Pascal `Text`.
#[derive(Default)]
pub struct PText {
    name: Option<PString>,
    mode: TextMode,
    /// Set by `AssignCrt`: opening the file opens the console.
    crt: bool,
}

#[derive(Default)]
enum TextMode {
    #[default]
    Closed,
    Reading(LineReader),
    Writing(BufWriter<File>),
    /// After `AssignCrt(f)` and `Reset`/`Rewrite`: the screen and keyboard,
    /// shared with plain `Write` and `Read`.
    Console,
    /// The printer, `Lst`, passed as a `Text` parameter.
    Printer,
}

impl PText {
    fn name(&self) -> &PString {
        match &self.name {
            Some(name) => name,
            None => crate::rte(102, "File not assigned"),
        }
    }

    /// Pascal `Assign(f, name)`.
    pub fn assign(&mut self, name: impl Into<PString>) {
        self.close_quietly();
        let name = name.into();
        // `Assign(f, '')` is standard input and output, as in TP: the same
        // console `AssignCrt` gives.
        self.crt = name.length() == 0;
        self.name = Some(name);
    }

    /// `Output` or `Input` passed as a `var f: Text` parameter: a file that's
    /// already open on the console.
    pub fn console() -> PText {
        PText { name: None, mode: TextMode::Console, crt: true }
    }

    /// `Lst` passed as a `var f: Text` parameter: a file that's already open
    /// on the printer.
    pub fn printer() -> PText {
        PText { name: None, mode: TextMode::Printer, crt: false }
    }

    /// Crt's `AssignCrt(f)`: like `Assign`, but for the screen and keyboard.
    pub fn assign_crt(&mut self) {
        self.close_quietly();
        self.name = None;
        self.crt = true;
    }

    /// Pascal `Reset(f)`: open for reading from the start.
    pub fn reset(&mut self) {
        self.close_quietly();
        if self.crt {
            self.mode = TextMode::Console;
            return;
        }
        let name = *self.name();
        match File::open(dos_path(&name.to_string())) {
            Ok(file) => self.mode = TextMode::Reading(LineReader::new(Source::File(BufReader::new(file)))),
            Err(e) => open_error(e, &name),
        }
    }

    /// Pascal `Rewrite(f)`: create (or empty) the file and open it for
    /// writing.
    pub fn rewrite(&mut self) {
        self.close_quietly();
        if self.crt {
            self.mode = TextMode::Console;
            return;
        }
        let name = *self.name();
        match File::create(dos_path(&name.to_string())) {
            Ok(file) => self.mode = TextMode::Writing(BufWriter::new(file)),
            Err(e) => open_error(e, &name),
        }
    }

    /// Pascal `Append(f)`: open for writing at the end.
    pub fn append(&mut self) {
        self.close_quietly();
        if self.crt {
            self.mode = TextMode::Console;
            return;
        }
        let name = *self.name();
        match OpenOptions::new().append(true).open(dos_path(&name.to_string())) {
            Ok(file) => self.mode = TextMode::Writing(BufWriter::new(file)),
            Err(e) => open_error(e, &name),
        }
    }

    /// Pascal `Close(f)`.
    pub fn close(&mut self) {
        if let TextMode::Closed = self.mode {
            crate::rte(103, "File not open");
        }
        self.close_quietly();
    }

    fn close_quietly(&mut self) {
        if let TextMode::Writing(writer) = &mut self.mode {
            let _ = writer.flush();
        }
        self.mode = TextMode::Closed;
    }

    /// Pascal `Flush(f)`.
    pub fn flush(&mut self) {
        match &mut self.mode {
            TextMode::Writing(writer) => {
                let _ = writer.flush();
            }
            TextMode::Console => super::flush_stdout(),
            TextMode::Printer => crate::printer::Lst.flush(),
            _ => {}
        }
    }

    /// Pascal `Erase(f)`: delete the (closed) file.
    pub fn erase(&mut self) {
        let name = *self.name();
        if let Err(e) = remove_file(&dos_path(&name.to_string())) {
            open_error(e, &name);
        }
    }

    /// Pascal `Rename(f, new_name)`.
    pub fn rename(&mut self, new_name: impl Into<PString>) {
        let (name, new_name) = (*self.name(), new_name.into());
        if let Err(e) = std::fs::rename(dos_path(&name.to_string()), dos_path(&new_name.to_string())) {
            open_error(e, &name);
        }
        self.name = Some(new_name);
    }

    fn reader(&mut self) -> &mut LineReader {
        match &mut self.mode {
            TextMode::Reading(reader) => reader,
            TextMode::Closed => crate::rte(103, "File not open"),
            TextMode::Writing(_) => crate::rte(104, "File not open for input"),
            TextMode::Console => unreachable!("console reads go to standard input"),
            TextMode::Printer => crate::rte(104, "File not open for input: the printer"),
        }
    }

    fn is_console(&self) -> bool {
        matches!(self.mode, TextMode::Console)
    }

    /// Pascal `Eof(f)`.
    pub fn eof(&mut self) -> bool {
        if self.is_console() { super::eof() } else { self.reader().eof() }
    }

    /// Pascal `Eoln(f)`.
    pub fn eoln(&mut self) -> bool {
        if self.is_console() { super::eoln() } else { self.reader().eoln() }
    }

    /// Pascal `SeekEof(f)`.
    pub fn seek_eof(&mut self) -> bool {
        if self.is_console() { super::seek_eof() } else { self.reader().seek_eof() }
    }

    /// Pascal `SeekEoln(f)`.
    pub fn seek_eoln(&mut self) -> bool {
        if self.is_console() { super::seek_eoln() } else { self.reader().seek_eoln() }
    }

    /// One value for `Read(f, ...)`; used by the `fread!` macro.
    pub fn read_into<T: PasRead>(&mut self, target: &mut T) {
        if self.is_console() {
            super::read_into(target);
        } else {
            target.pas_read(self.reader());
        }
    }

    /// The end of a `Readln(f, ...)`.
    pub fn readln_end(&mut self) {
        if self.is_console() {
            super::readln_end();
        } else {
            self.reader().end_line();
        }
    }

    /// Text for `Write(f, ...)`; used by the `fwrite!` macro.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        match &mut self.mode {
            TextMode::Writing(writer) => {
                if writer.write_all(bytes).is_err() {
                    crate::rte(101, "Disk write error");
                }
            }
            TextMode::Console => super::write_stdout(bytes),
            TextMode::Printer => crate::printer::Lst.write_bytes(bytes),
            TextMode::Closed => crate::rte(103, "File not open"),
            TextMode::Reading(_) => crate::rte(105, "File not open for output"),
        }
    }
}

/// Copying a Pascal file variable copies its name; the open handle can't be
/// shared, so the copy starts out closed. (This is only needed so that
/// records containing a `Text` can derive `Clone`.)
impl Clone for PText {
    fn clone(&self) -> PText {
        PText { name: self.name, mode: TextMode::Closed, crt: self.crt }
    }
}

impl std::fmt::Debug for PText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PText({:?})", self.name)
    }
}

impl Drop for PText {
    fn drop(&mut self) {
        self.close_quietly();
    }
}

/// Pascal `Write(f, a, b, ...)` to a text file.
#[macro_export]
macro_rules! fwrite {
    ($file:expr $(, $arg:expr)* $(,)?) => {{
        // Each value is written before the next is worked out, as with
        // `pwrite!`: a function in the list that writes to the same file
        // comes out in its place.
        $({
            let mut buf: ::std::vec::Vec<u8> = ::std::vec::Vec::new();
            $crate::io::PasWrite::pas_write(&$arg, &mut buf);
            $file.write_bytes(&buf);
        })*
    }};
}

/// Pascal `Writeln(f, a, b, ...)` to a text file.
#[macro_export]
macro_rules! fwriteln {
    ($file:expr $(, $arg:expr)* $(,)?) => {{
        $crate::fwrite!($file $(, $arg)*);
        $file.write_bytes(b"\n");
    }};
}

/// Pascal `Read(f, a, b, ...)` from a text file.
#[macro_export]
macro_rules! fread {
    ($file:expr $(, $arg:expr)* $(,)?) => {{
        $( $file.read_into($arg); )*
    }};
}

/// Pascal `Readln(f, a, b, ...)` from a text file.
#[macro_export]
macro_rules! freadln {
    ($file:expr $(, $arg:expr)* $(,)?) => {{
        $( $file.read_into($arg); )*
        $file.readln_end();
    }};
}

// ----- Typed files ----------------------------------------------------------

/// A value that can be stored in a `file of T`: it knows how to turn itself
/// into bytes and back. Records get an implementation from the
/// [`file_record!`](crate::file_record) macro.
///
/// The byte layout is our own (little-endian numbers, 256 bytes per string),
/// not Turbo Pascal's, so files written by the original DOS programs can't be
/// read. Every value of a type must encode to the same number of bytes, so
/// that `Seek` can find record `n` at byte `n * size`.
pub trait FileRecord: Default {
    fn encode(&self, out: &mut Vec<u8>);
    /// Decode a value from the front of `input`, advancing it past the bytes
    /// used.
    fn decode(input: &mut &[u8]) -> Self;

    /// The size of one record, found by encoding the default value.
    fn record_size() -> usize {
        let mut bytes = Vec::new();
        Self::default().encode(&mut bytes);
        bytes.len()
    }
}

/// Take `n` bytes from the front of `input`.
fn take<'a>(input: &mut &'a [u8], n: usize) -> &'a [u8] {
    let (head, tail) = input.split_at(n);
    *input = tail;
    head
}

macro_rules! impl_file_record_number {
    ($($t:ty),*) => {
        $( impl FileRecord for $t {
            fn encode(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
            fn decode(input: &mut &[u8]) -> $t {
                // `try_into` turns the slice into the fixed-size array that
                // `from_le_bytes` wants; `take` guarantees the length.
                <$t>::from_le_bytes(take(input, std::mem::size_of::<$t>()).try_into().unwrap())
            }
        } )*
    };
}
impl_file_record_number!(i8, i16, i32, i64, u8, u16, u32, f32);

/// `Real` is stored in Turbo Pascal's own 6-byte format (PascRust's `Real`,
/// `Single`, `Double` and `Extended` are all an `f64`, and TP's `Real` is
/// what its programs' files hold). See [`real48`].
impl FileRecord for f64 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&real48::from_f64(*self));
    }
    fn decode(input: &mut &[u8]) -> f64 {
        real48::to_f64(take(input, 6).try_into().unwrap())
    }
}

/// Turbo Pascal's 6-byte `Real`: byte 0 is the exponent, biased by 129 (0
/// means the value is 0); bytes 1 to 5 hold 39 bits of mantissa, low byte
/// first, with the sign in the top bit of byte 5. The value is
/// `(1 + mantissa / 2^39) * 2^(exponent - 129)`. It has about 11-12 digits,
/// so an `f64` stored and read back is rounded to that, as in TP.
pub mod real48 {
    pub fn from_f64(x: f64) -> [u8; 6] {
        if x == 0.0 || !x.is_finite() {
            return [0; 6];
        }
        let bits = x.to_bits();
        let sign = (bits >> 63) as u8;
        let exponent = ((bits >> 52) & 0x7FF) as i64 - 1023 + 129;
        // The f64 mantissa's top 39 of 52 bits, rounded.
        let mut mantissa = ((bits & ((1 << 52) - 1)) + (1 << 12)) >> 13;
        let mut exponent = exponent;
        if mantissa == 1 << 39 {
            mantissa = 0;
            exponent += 1;
        }
        if exponent < 1 {
            return [0; 6]; // too small for a Real: 0
        }
        let exponent = exponent.min(255) as u8;
        let mut out = [0u8; 6];
        out[0] = exponent;
        for (i, byte) in out[1..].iter_mut().enumerate() {
            *byte = (mantissa >> (8 * i)) as u8;
        }
        out[5] |= sign << 7;
        out
    }

    pub fn to_f64(bytes: [u8; 6]) -> f64 {
        if bytes[0] == 0 {
            return 0.0;
        }
        let mut mantissa: u64 = 0;
        for (i, &byte) in bytes[1..].iter().enumerate() {
            mantissa |= u64::from(byte) << (8 * i);
        }
        let negative = mantissa & (1 << 39) != 0;
        mantissa &= (1 << 39) - 1;
        let value = (1.0 + mantissa as f64 / (1u64 << 39) as f64) * 2f64.powi(i32::from(bytes[0]) - 129);
        if negative { -value } else { value }
    }
}

impl FileRecord for bool {
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(*self as u8);
    }
    fn decode(input: &mut &[u8]) -> bool {
        take(input, 1)[0] != 0
    }
}

/// A plain `string` takes 256 bytes; a `string[n]` field takes n+1, which
/// [`FileString`] does (`file_record!(Rec { name: string 30 })`).
impl FileRecord for PString {
    fn encode(&self, out: &mut Vec<u8>) {
        self.encode_string(255, out);
    }
    fn decode(input: &mut &[u8]) -> PString {
        PString::decode_string(255, input)
    }
}

/// Strings stored with a capacity: a `string[30]` is a length byte and 30
/// characters, as in TP, and so is each string in an array of them.
pub trait FileString: Sized {
    fn encode_string(&self, capacity: usize, out: &mut Vec<u8>);
    fn decode_string(capacity: usize, input: &mut &[u8]) -> Self;
}

impl FileString for PString {
    fn encode_string(&self, capacity: usize, out: &mut Vec<u8>) {
        let text = &self.as_bytes()[..self.as_bytes().len().min(capacity)];
        out.push(text.len() as u8);
        out.extend_from_slice(text);
        // TP wrote whatever was in memory after the text; zeros here.
        out.resize(out.len() + capacity - text.len(), 0);
    }
    fn decode_string(capacity: usize, input: &mut &[u8]) -> PString {
        let bytes = take(input, capacity + 1);
        // A length byte from another program may claim more than fits.
        let length = usize::from(bytes[0]).min(capacity);
        PString::from_bytes(&bytes[1..=length])
    }
}

impl<T: FileString, const LO: i64, const N: usize> FileString for crate::array::PArray<T, LO, N> {
    fn encode_string(&self, capacity: usize, out: &mut Vec<u8>) {
        for item in self.iter() {
            item.encode_string(capacity, out);
        }
    }
    fn decode_string(capacity: usize, input: &mut &[u8]) -> Self {
        crate::array::PArray::new(std::array::from_fn(|_| T::decode_string(capacity, input)))
    }
}

impl<T: FileRecord, const LO: i64, const N: usize> FileRecord for crate::array::PArray<T, LO, N> {
    fn encode(&self, out: &mut Vec<u8>) {
        for item in self.iter() {
            item.encode(out);
        }
    }
    fn decode(input: &mut &[u8]) -> Self {
        crate::array::PArray::new(std::array::from_fn(|_| T::decode(input)))
    }
}

/// A pointer is just an address, meaningless once the program ends; Turbo
/// Pascal programs that store records containing pointers never use the
/// stored pointers. They're written as 4 zero bytes and read back as `nil`.
impl<T> FileRecord for crate::ptr::Ptr<T> {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[0; 4]);
    }
    fn decode(input: &mut &[u8]) -> Self {
        take(input, 4);
        crate::ptr::Ptr::nil()
    }
}

/// Implement [`FileRecord`] for a record (struct) by encoding its fields in
/// order, or for an enum by its ordinal. A `string[n]` field (or an array
/// of them) says its capacity:
///
/// ```ignore
/// file_record!(Person { name: string 30, age });
/// file_record!(enum Color);
/// ```
#[macro_export]
macro_rules! file_record {
    (enum $ty:ident) => {
        impl $crate::io::FileRecord for $ty {
            fn encode(&self, out: &mut ::std::vec::Vec<u8>) {
                out.push($crate::ordinal::Ordinal::ord(*self) as u8);
            }
            fn decode(input: &mut &[u8]) -> Self {
                // A byte beyond the last value (another program's file) is
                // read as the last value rather than stopping the program.
                let values = <$ty as $crate::ordinal::Enumerated>::VALUES;
                let ord = usize::from(<u8 as $crate::io::FileRecord>::decode(input));
                values[ord.min(values.len() - 1)]
            }
        }
    };
    ($ty:ident { $($field:ident $(: $kind:ident $size:literal)?),* $(,)? }) => {
        impl $crate::io::FileRecord for $ty {
            fn encode(&self, out: &mut ::std::vec::Vec<u8>) {
                $( $crate::file_record!(@encode self.$field, out $(, $kind $size)?); )*
            }
            fn decode(input: &mut &[u8]) -> Self {
                // Struct fields are evaluated in the order written, which is
                // the order they were encoded in.
                $ty { $( $field: $crate::file_record!(@decode input $(, $kind $size)?), )* }
            }
        }
    };
    // One field. `string 30` is a `string[30]` (or an array of them); `set 1`
    // is a set stored in its first byte (a set of an enum of up to 8 values).
    (@encode $value:expr, $out:ident) => {
        $crate::io::FileRecord::encode(&$value, $out)
    };
    (@encode $value:expr, $out:ident, string $cap:literal) => {
        $crate::io::FileString::encode_string(&$value, $cap, $out)
    };
    (@encode $value:expr, $out:ident, set $bytes:literal) => {
        $value.encode_bytes($bytes, $out)
    };
    (@decode $input:ident) => {
        $crate::io::FileRecord::decode($input)
    };
    (@decode $input:ident, string $cap:literal) => {
        $crate::io::FileString::decode_string($cap, $input)
    };
    (@decode $input:ident, set $bytes:literal) => {
        $crate::set::PSet::decode_bytes($bytes, $input)
    };
}

/// Pascal `file of T`.
pub struct PFile<T> {
    name: Option<PString>,
    file: Option<File>,
    // `PhantomData` says "this type is about `T`" without storing a `T`.
    _record: PhantomData<T>,
}

impl<T> Default for PFile<T> {
    fn default() -> Self {
        PFile { name: None, file: None, _record: PhantomData }
    }
}

/// As for [`PText`], a copy is closed.
impl<T> Clone for PFile<T> {
    fn clone(&self) -> Self {
        PFile { name: self.name, file: None, _record: PhantomData }
    }
}

impl<T> std::fmt::Debug for PFile<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PFile({:?})", self.name)
    }
}

impl<T: FileRecord> PFile<T> {
    fn name(&self) -> PString {
        match self.name {
            Some(name) => name,
            None => crate::rte(102, "File not assigned"),
        }
    }

    fn file(&mut self) -> &mut File {
        match &mut self.file {
            Some(file) => file,
            None => crate::rte(103, "File not open"),
        }
    }

    /// The open file, shared. `&File` implements `Seek` too, which lets the
    /// size and position queries take `&self`, so that `Seek(f, FileSize(f))`
    /// doesn't need two mutable borrows of `f`.
    fn file_ref(&self) -> &File {
        match &self.file {
            Some(file) => file,
            None => crate::rte(103, "File not open"),
        }
    }

    /// Pascal `Assign(f, name)`.
    pub fn assign(&mut self, name: impl Into<PString>) {
        self.file = None;
        self.name = Some(name.into());
    }

    /// Pascal `Reset(f)`: open an existing file, for reading and writing
    /// unless `FileMode` says otherwise.
    pub fn reset(&mut self) {
        let name = self.name();
        // `FileMode`'s low two bits say how: 0 read-only, 1 write-only, 2
        // both. (TP 7 programs add DOS sharing flags above them, like
        // `FileMode := $40` for read-only, deny none.)
        let mode = crate::system::file_mode() & 3;
        match OpenOptions::new().read(mode != 1).write(mode != 0).open(dos_path(&name.to_string())) {
            Ok(file) => self.file = Some(file),
            Err(e) => open_error(e, &name),
        }
    }

    /// Pascal `Rewrite(f)`: create an empty file for reading and writing.
    pub fn rewrite(&mut self) {
        let name = self.name();
        let path = dos_path(&name.to_string());
        match OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path) {
            Ok(file) => self.file = Some(file),
            Err(e) => open_error(e, &name),
        }
    }

    /// Pascal `Close(f)`.
    pub fn close(&mut self) {
        if self.file.take().is_none() {
            crate::rte(103, "File not open");
        }
    }

    /// Pascal `Erase(f)`.
    pub fn erase(&mut self) {
        let name = self.name();
        if let Err(e) = remove_file(&dos_path(&name.to_string())) {
            open_error(e, &name);
        }
    }

    /// Pascal `Rename(f, new_name)`.
    pub fn rename(&mut self, new_name: impl Into<PString>) {
        let (name, new_name) = (self.name(), new_name.into());
        if let Err(e) = std::fs::rename(dos_path(&name.to_string()), dos_path(&new_name.to_string())) {
            open_error(e, &name);
        }
        self.name = Some(new_name);
    }

    /// Pascal `FileSize(f)`: the number of records.
    pub fn file_size(&self) -> i32 {
        let bytes = self.file_ref().metadata().map_or(0, |m| m.len());
        (bytes / T::record_size() as u64) as i32
    }

    /// Pascal `FilePos(f)`: the number of the next record (0-based).
    pub fn file_pos(&self) -> i32 {
        let pos = self.file_ref().stream_position().unwrap_or(0);
        (pos / T::record_size() as u64) as i32
    }

    /// Pascal `Seek(f, n)`: move to record `n` (0-based).
    pub fn seek(&mut self, n: impl Into<i32>) {
        let n = n.into();
        if n < 0 || n > self.file_size() {
            crate::rte(100, format!("Disk read error: seek to record {n}"));
        }
        let offset = n as u64 * T::record_size() as u64;
        if self.file().seek(SeekFrom::Start(offset)).is_err() {
            crate::rte(100, "Disk read error");
        }
    }

    /// Pascal `Eof(f)`: is the position at the end?
    pub fn eof(&self) -> bool {
        self.file_pos() >= self.file_size()
    }

    /// Pascal `Truncate(f)`: cut the file off at the current position.
    pub fn truncate(&mut self) {
        let pos = self.file().stream_position().unwrap_or(0);
        let _ = self.file().set_len(pos);
    }

    /// Pascal `Read(f, x)`: read the next record.
    pub fn read(&mut self, target: &mut T) {
        let mut bytes = vec![0; T::record_size()];
        if self.file().read_exact(&mut bytes).is_err() {
            crate::rte(100, "Disk read error: read past the end of the file");
        }
        *target = T::decode(&mut bytes.as_slice());
    }

    /// Pascal `Write(f, x)`: write a record at the current position.
    pub fn write(&mut self, value: &T) {
        let mut bytes = Vec::new();
        value.encode(&mut bytes);
        if self.file().write_all(&bytes).is_err() {
            crate::rte(101, "Disk write error");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug, PartialEq, Clone)]
    struct Person {
        name: PString,
        age: i16,
    }
    crate::file_record!(Person { name, age });

    fn temp_path(name: &str) -> String {
        std::env::temp_dir().join(format!("pascrust-{}-{name}", std::process::id())).to_string_lossy().into_owned()
    }

    #[test]
    fn typed_file_round_trip_and_seek() {
        let path = temp_path("people.dat");
        let mut f: PFile<Person> = PFile::default();
        f.assign(path.as_str());
        f.rewrite();
        for (i, name) in ["Ann", "Bob", "Cy"].iter().enumerate() {
            f.write(&Person { name: PString::from(*name), age: i as i16 * 10 });
        }
        assert_eq!(f.file_size(), 3);
        f.seek(1);
        let mut p = Person::default();
        f.read(&mut p);
        assert_eq!(p, Person { name: PString::from("Bob"), age: 10 });
        assert_eq!(f.file_pos(), 2);
        f.close();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn text_file_round_trip() {
        let path = temp_path("notes.txt");
        let mut t = PText::default();
        t.assign(path.as_str());
        t.rewrite();
        crate::fwriteln!(t, "x = ", 42i16);
        crate::fwriteln!(t, "second line");
        t.close();
        t.reset();
        let mut word = PString::new();
        t.read_into(&mut word);
        assert_eq!(word, "x = 42");
        t.readln_end();
        t.read_into(&mut word);
        assert_eq!(word, "second line");
        t.readln_end();
        assert!(t.eof());
        t.close();
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    /// The record of `examples/database/founit.pas`.
    #[derive(Default)]
    struct People {
        name: PString,
        id: i32,
        index: f64,
    }
    crate::file_record!(People { name: string 30, id, index });

    #[test]
    fn records_are_laid_out_as_in_turbo_pascal() {
        // `mark.dat` was written by the real program, under TP: a
        // `string[30]` (31 bytes), a `LongInt` (4) and a `Real` (6).
        let bytes = include_bytes!("../../../../examples/database/mark.dat");
        assert_eq!(People::record_size(), 41);
        let person = People::decode(&mut &bytes[..]);
        assert_eq!(person.name.to_string(), "Shannon Behrens");
        assert_eq!((person.id, person.index), (1, 2930802.0));
        let mut again = Vec::new();
        person.encode(&mut again);
        assert_eq!(again.len(), 41);
        assert_eq!(People::decode(&mut &again[..]).index, 2930802.0);
    }

    #[test]
    fn reals_round_trip_to_about_twelve_digits() {
        for x in [1.0, -1.0, 0.5, 1.23456789012, 1e30, -2.5e-30, 123456789.125] {
            let back = real48::to_f64(real48::from_f64(x));
            assert!(((back - x) / x).abs() < 1e-11, "{x} came back as {back}");
        }
        assert_eq!(real48::from_f64(0.0), [0; 6]);
        assert_eq!(real48::from_f64(1.0), [129, 0, 0, 0, 0, 0]);
        assert_eq!(real48::from_f64(-2.0), [130, 0, 0, 0, 0, 0x80]);
    }

    #[test]
    fn a_string_longer_than_its_length_byte_allows_is_cut() {
        let mut bytes = vec![200u8];
        bytes.extend_from_slice(&[b'x'; 30]);
        assert_eq!(PString::decode_string(30, &mut &bytes[..]).length(), 30);
    }
}
