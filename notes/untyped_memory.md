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
   `var` parameters already have to handle (architecture.md, §15).

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
