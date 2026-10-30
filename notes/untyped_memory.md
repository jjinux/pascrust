# Untyped memory: `GetMem`, `Pointer`, untyped `var` and untyped files

A proposal, for the user to decide on before it's built. The rest of
Phase 7 needs it:

- `GetImage`/`PutImage`/`ImageSize`: programs allocate the buffer with
  `GetMem(p, ImageSize(...))` and pass `p^`.
- Untyped files: `BlockRead(f, buf, count)`/`BlockWrite` read and write
  any variable as bytes.
- `PChar` and the `Strings` unit: a `PChar` is a pointer into a byte
  buffer, and `PChar` arithmetic is pointer arithmetic.
- And beyond Phase 7, `Move`/`FillChar` between different types, which
  PascRust refuses today.

All of these treat a variable as bytes. TP did that by handing out its
address. PascRust can't do that safely in Rust, so the question is what
to do instead.

## What PascRust has already

- Typed files (`file of T`) write records in TP's byte layout, through the
  runtime's `FileRecord` trait: `encode(&self, out: &mut Vec<u8>)` and
  `decode(input) -> Self`, generated for every record stored in a file.
  So "this variable as TP's bytes" exists for records, arrays, strings and
  numbers.
- Typed pointers are `Ptr<T>`, a shared, nullable box
  (`Option<Rc<RefCell<...>>>`); `New`/`Dispose` work on them.

## Proposal

1. **Byte views.** A trait in the runtime, say `ByteView`, with
   `to_bytes(&self) -> Vec<u8>` and `from_bytes(&mut self, &[u8])`, built
   on `FileRecord`'s encoding and implemented for every type the program
   uses as untyped memory. An untyped `var` parameter
   (`procedure Save(var Data; Size: Word)`, `BlockRead(f, Buf, ...)`)
   becomes `&mut dyn ByteView`. A routine that takes one copies the bytes
   in, works on them, and copies them back. This is not aliasing (a
   change through the view only appears when the routine returns). A
   program can only tell if the routine also reaches the variable another
   way (as a global, or the same variable passed twice), the same case
   `var` parameters already have to handle (docs/architecture.md, §15).

2. **`Pointer` and `GetMem`.** An untyped `Pointer` becomes `PBuf`: a
   shared byte buffer (`Rc<RefCell<Vec<u8>>>`) plus an offset, and `nil`.
   `GetMem(p, n)` allocates `n` zero bytes; `FreeMem` drops them. `p^` is
   allowed only where a byte view is wanted (an untyped `var` argument);
   `PBuf` is a `ByteView` of its bytes. A typed pointer given memory by
   `GetMem(p, SizeOf(T))` is the same as `New(p)`, and becomes that.

3. **Untyped files.** `file` (without `of`) becomes `PFile<u8>` with a
   record size (128 bytes by default; `Reset(f, 1)` and `Rewrite(f, 1)`
   set it), and `BlockRead(f, Buf, Count, Result)`/`BlockWrite` move
   `Count` records between the file and `Buf`'s byte view.

4. **`GetImage`/`PutImage`.** The image is stored in the buffer as PascRust
   likes: the width and height as two `Word`s (as TP did), then one byte
   per pixel (TP stored the VGA's four bit planes). `ImageSize` reports
   what that needs. A program that only hands the buffer back to
   `PutImage` can't tell; one that reads the pixels out of the buffer
   itself would see different bytes.

5. **`PChar`.** A `PBuf` whose bytes end in a `#0`; a string literal
   passed as a `PChar` becomes one; `p + n` and `p[i]` move the offset or
   read a byte. The `Strings` unit (`StrLen`, `StrCopy`, `StrCat`,
   `StrComp`, `StrPas`, `StrPCopy`, ...) works on these, and `WinDos` is
   then mostly aliases of `Dos`.

## What would still be refused

- `Addr(x)`/`@x` of an ordinary variable, and `Ptr(Seg, Ofs)`/`Mem[]`:
  real addresses. (`@x` passed straight to an untyped `var` could be
  allowed: it's the same byte view.)
- Variant records used to reinterpret bytes (`absolute` over different
  types): the byte view could do it, but it's a bigger change.

## Questions for the user

1. Are byte views that copy in and out at the call acceptable, rather than
   true aliasing?
2. For `GetImage`, is PascRust's own layout in the buffer fine, given that
   a program only sees it if it reads the buffer's bytes itself?
3. Which first: untyped files with `BlockRead`/`BlockWrite`, or
   `GetImage`/`PutImage`? Both need parts 1 and 2.

## Fable's review (2026-10-29)

Verdict: the shape is right (TP's bytes through a runtime trait,
copy-in/copy-out at the untyped `var`, a byte buffer for `GetMem`, a block
file over it), buildable without `unsafe`. But:

1. The byte view must match TP's `SizeOf`, not `FileRecord`'s current
   layout (which writes a `string[10]` as 256 bytes and a `Real` as 8).
   So first make `FileRecord` TP-exact: `string[n]` as n+1 bytes (codegen
   passes capacities to `file_record!`), TP's 6-byte `Real`, 4/8/10/8 for
   `Single`/`Double`/`Extended`/`Comp`, sets by byte range. That also makes
   `examples/database/mark.dat` readable.
2. `Pointer` is also TP's `void*`: lists of `Pointer`, `PNode(p)^.x`. Make
   it an enum: `Nil`, `Bytes(PBuf)` (from untyped `GetMem`), `Any(Rc<dyn
   Any>)` (from a typed pointer), with `p.cast::<Node>()` checked at run
   time.
3. Views must be able to start inside a variable: `BlockWrite(f, s[1],
   Length(s))`, `BlockRead(f, Buf[pos], n)`.
4. A typed `GetMem(p, n)` with `n < SizeOf(T)` is the dynamic-array idiom:
   allocate the full declared type, and indexes past `n` are range errors.
5. `BlockRead` may read short: write back only the bytes read, and
   `decode` must accept any bytes (clamp a string's length byte, an enum
   out of range).
6. Copy out on every exit, including `Exit`: a guard whose `Drop` writes
   back.
7. `var f: file` takes an untyped or typed file, not `Text`.

Answers to the questions:

1. Yes, copy-in/copy-out, under those conditions: an untyped parameter
   is only used through casts, `Move`/`FillChar` or runtime routines, and
   the other ways to reach the variable (a global, the heap) are already
   caught.
2. No: use TP's own layout (two `Word`s, width-1 and height-1, then the
   VGA's four bit planes), so hand-sized static buffers, programs that
   read the header, and sprite files from the original programs all
   work.
3. Untyped files first: they need only the layout and byte views, not
   `Pointer`, and they're the most common.

Build order: (0) TP-exact layout; (1) byte views, untyped `var`,
`Move`/`FillChar` between types; (2) untyped files and `BlockRead`/
`BlockWrite`; (3) `Pointer`, `GetMem`/`FreeMem`; (4) `GetImage`/
`PutImage`; (5) `PChar` and `Strings`.

## Step 5 as built (2026-10-29)

`PChar` is a byte buffer plus an offset, as planned. The open question was
TP 7's `P := A`, a `PChar` kept into a zero-based `Char` array, which
needs a pointer into an ordinary variable. The user chose "copy at the
call" first: the statement that uses an array as a `PChar` gets a copy of
its characters, written back after it, and `sema/pchar.rs` refuses any
`PChar` into an array that would outlive the statement (`P := A`,
returning it, a routine that keeps its parameter). Making the arrays
shared buffers (so `P := A` aliases) is in TODO.md for when a real
program needs it. The rules come from the TP 7 Language Guide, chapter 16
and table 6.7.
