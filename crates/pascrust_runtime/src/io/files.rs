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
        self.name = Some(name.into());
        self.crt = false;
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
        if let Err(e) = std::fs::remove_file(dos_path(&name.to_string())) {
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
        let mut buf: ::std::vec::Vec<u8> = ::std::vec::Vec::new();
        $( $crate::io::PasWrite::pas_write(&$arg, &mut buf); )*
        $file.write_bytes(&buf);
    }};
}

/// Pascal `Writeln(f, a, b, ...)` to a text file.
#[macro_export]
macro_rules! fwriteln {
    ($file:expr $(, $arg:expr)* $(,)?) => {{
        let mut buf: ::std::vec::Vec<u8> = ::std::vec::Vec::new();
        $( $crate::io::PasWrite::pas_write(&$arg, &mut buf); )*
        buf.push(b'\n');
        $file.write_bytes(&buf);
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
impl_file_record_number!(i8, i16, i32, i64, u8, u16, u32, f32, f64);

impl FileRecord for bool {
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(*self as u8);
    }
    fn decode(input: &mut &[u8]) -> bool {
        take(input, 1)[0] != 0
    }
}

impl FileRecord for PString {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = [0u8; 256];
        bytes[0] = self.length() as u8;
        bytes[1..=self.as_bytes().len()].copy_from_slice(self.as_bytes());
        out.extend_from_slice(&bytes);
    }
    fn decode(input: &mut &[u8]) -> PString {
        let bytes = take(input, 256);
        PString::from_bytes(&bytes[1..=bytes[0] as usize])
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
/// order, or for an enum by its ordinal:
///
/// ```ignore
/// file_record!(Person { name, age });
/// file_record!(enum Color);
/// ```
#[macro_export]
macro_rules! file_record {
    (enum $ty:ident) => {
        impl $crate::io::FileRecord for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                out.push($crate::ordinal::Ordinal::ord(*self) as u8);
            }
            fn decode(input: &mut &[u8]) -> Self {
                let ord = <u8 as $crate::io::FileRecord>::decode(input);
                <$ty as $crate::ordinal::Enumerated>::from_ord(ord)
            }
        }
    };
    ($ty:ident { $($field:ident),* $(,)? }) => {
        impl $crate::io::FileRecord for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                $( $crate::io::FileRecord::encode(&self.$field, out); )*
            }
            fn decode(input: &mut &[u8]) -> Self {
                // Struct fields are evaluated in the order written, which is
                // the order they were encoded in.
                $ty { $( $field: $crate::io::FileRecord::decode(input), )* }
            }
        }
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
        if let Err(e) = std::fs::remove_file(dos_path(&name.to_string())) {
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
