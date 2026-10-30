# Variant records and procedural types

A proposal, for the user to decide on before it's built. Both are tier 3
in `docs/language_spec.md` (items 2 and 4), both are refused today
(`variant records are not supported yet`; `expected an expression, found
`function``), and neither is used by the programs in `examples/` (checked:
no `case` inside a record, no `procedure(`/`function(` in a type section,
and `@` appears only inside string literals). So this is for other old
programs: the Turbo Pascal idioms that these two features carry are
common, though (a tagged union for an expression tree or a shape; a
`LongInt` read as two `Word`s; a `Plot(F)` or a sort with a compare
function).

Scratch programs checked against `fpc -Mtp` are in `out/review/design/`
(`vr*.pas`, `pt*.pas`).

## What Turbo Pascal does

From the TP 7 Language Guide (`out/ref/guide.txt`).

**Variant records** (chapter 4, "Record types", pp. 32-33): a record is a
fixed part followed by an optional variant part, `case [Tag:] OrdinalType
of label, ...: (fields); ...`. "The variants overlay the same space in
memory, and all fields of all variants can be accessed at all times."
"If a tag field identifier is present, it becomes the identifier of an
additional fixed field, the tag field, of the record. The program can use
the tag field's value to show which variant is active at a given time.
Without a tag field, the program selects a variant by another criterion."
Each variant's field list may itself end in a variant part (the syntax
diagram is recursive). Chapter 19, "Memory issues" (p. 221): "If the
record contains variant parts, then each variant starts at the same
memory address", so `SizeOf` is the fixed part plus the largest variant,
unpadded. Chapter 5, typed constants (p. 61): "If a record contains a
variant, only fields of the selected variant can be specified. If the
variant contains a tag field, then its value must be specified." Nothing
checks the tag at run time (`{$R+}` doesn't either), which is why the
same syntax serves two purposes: a tagged union (`TPolygon` with `case
Kind: Figure of`, p. 33) and type punning (`case Integer of 0: (L:
LongInt); 1: (Lo, Hi: Word)`, as in the System unit's `LongRec`).

**Procedural types** (chapter 4, "Procedural types", pp. 43-46): `type
Proc = procedure; MathFunc = function(X: Real): Real; SwapProc =
procedure(var X, Y: Integer)`; parameter names are decorative. A
procedural value is `nil`, a variable of a procedural type, or a
procedure/function identifier; `P := Swap; F := Tan; P(I, J); X := F(X)`.
"Using a procedural variable that has been assigned the value nil in a
procedure statement or a function call results in an error"; guard with
`if @P <> nil`. Compatibility (pp. 45, 47-48): same number of parameters,
identical parameter types in order, identical result type; `nil` fits any.
Restrictions: a routine used as a value must be `far` or compiled under
`{$F+}` (chapter 8, p. 98); "standard procedures and functions, nested
procedures and functions, methods, inline procedures and functions, and
interrupt procedures can't be used as procedural values" (a standard one
needs a "shell" such as `FSin`). "Turbo Pascal doesn't let you declare
functions that return procedural-type values" (p. 44). Chapter 6,
"Procedural types in expressions" (pp. 78-79): a procedural variable in a
statement or expression *calls* the routine, except on the left of `:=`
when the target is procedural, so `F := ReadInt` stores and `N := ReadInt`
calls; `if F = ReadInt` calls both; `@F = @ReadInt` compares addresses,
`@F` being an untyped `Pointer`. Typed constants (p. 64): `const
ErrorHandler: TErrorProc = DefaultError;`. Typecasts (p. 57): `F :=
Func(P)`, `Func(P) := F`, `@F := P`, `P := @F`, `N := Func(P)(N)`.
`Assigned(P)` (chapter 15) takes a pointer or a procedural variable.

Free Pascal in `-Mtp` mode agrees on all of this except the `far`
requirement (it ignores it), and its layout differs where a `Real` is
involved (an 8-byte `Double`, aligned: `SizeOf` of a record with a `Real`
variant is 16 where TP says 11) and `SizeOf(Func)` is 8 (TP: 4).

## What PascRust has already

- Records are structs with `derive(Clone, Default, Debug)` (`Copy` when
  every field is), and `docs/architecture.md` §11 planned variant records
  as *flattened* structs: every variant's fields become ordinary fields,
  with punning and `SizeOf` unsupported. That plan predates the byte
  layout work, which changes the answer below.
- TP's byte layout of every storable type: `FileRecord` (`encode`/`decode`
  in `runtime/src/io/files.rs`), generated per record by `file_record!`,
  with `string[n]` as n+1 bytes, `Real` as the 6-byte real48, sets by
  byte range. `ByteView` (`runtime/src/bytes.rs`) builds untyped `var`
  parameters, `Move`/`FillChar`, `BlockRead`/`BlockWrite` and `GetMem`
  memory on it, with the `Bytes` guard that copies in and writes back on
  `Drop`. `tp_size` in `sema/types.rs` is what `SizeOf` returns.
- Lifting (`sema/lift.rs`): a routine receives every global it or anything
  it calls touches, as `&mut T` / `&T` / by value, computed as a fixed
  point over `ProcInfo::calls`. Virtual methods already solve "several
  routines must share one Rust signature": `sema/mod.rs::dispatch` makes
  every version of a virtual method "call" the others, so `lift` gives
  them the union of their globals.
- `@Handler` already exists for `SetIntVec`: `AnyPtr::Code(name)`, a
  `Pointer` variant that is only stored and compared by name.
- `with` is a rewrite (every field written out as `r.field`), so it needs
  nothing from either feature.

## Proposal 1: variant records

Two representations, chosen per record type from its declaration, both
with TP's exact `SizeOf` and byte layout (so `file of Rec`, untyped `var`,
`Move`/`FillChar` and `BlockRead`/`BlockWrite` all see TP's bytes):

**A tagged variant part (`case Kind: Shape of`) is flattened.** The tag is
a field, and every variant's fields are ordinary struct fields, as §11
planned. This is the tagged-union use, and the output stays plain Rust:
`f.kind = Shape::Rect; f.w = 3;`. The byte layout comes from the tag:
`encode` writes the fixed fields, the tag, then the fields of the variant
the tag's value selects, zero-padded to the size of the largest variant;
`decode` reads the tag and fills that variant, leaving the others at
their defaults. What differs from TP: reading a field of a variant other
than the one last written gives that field's own last value (or zero),
not the other variant's bytes. A program that puns through a tagged
record is contradicting its own declaration; this is documented, not
detected (see question 1 for a heuristic).

**A tagless variant part (`case Integer of`) is kept as bytes.** This is
the punning use, and there is no other way to get it right. The struct
gets one field, `variants: VariantPart<N>` (N = the largest variant's
TP size, `[u8; N]` inside, `Copy`, `Default` zeros), and each variant
field becomes three small methods that read or write its slice of those
bytes through the `FileRecord` encoding. `Variants` is a `ByteView`
already, so nothing else changes.

Exception: a tagless variant part holding a field that can't be bytes (a
pointer, a file, an object with virtual methods) is flattened too, and
such a record can't be stored in a file or seen as bytes (error below).
Pointers in a union are the tagged-union idiom anyway (`case Kind:
NodeKind of nkOp: (Left, Right: PExpr)`).

Nested variant parts (a `case` at the end of a variant's field list, or an
anonymous record in a variant) follow the same rule recursively; the
outermost tagless part turns everything inside it into bytes.

### Generated code

```pascal
type
  Shape = (Circle, Rect);
  Fig = record
    X, Y: Integer;
    case Kind: Shape of
      Circle: (R: Real);
      Rect: (W, H: Integer);
  end;
  Pun = record
    case Integer of
      0: (L: LongInt);
      1: (Lo, Hi: Word);
      2: (B: array[1..4] of Byte);
  end;
var f: Fig; p: Pun; fl: file of Fig;
begin
  f.Kind := Rect; f.W := 3; f.H := 4;
  writeln(f.W * f.H, ' ', SizeOf(Fig));
  write(fl, f);
  p.L := $12345678;
  writeln(p.Lo, ' ', p.B[1]);
  Inc(p.Hi);
  p.B[4] := 0;
end.
```

```rust
/// Pascal: `Fig = record ... end`
///
/// The `case Kind: Shape of` part is a tagged union: the program sets `kind`
/// to say which fields are in use. Rust would use an `enum`, but Pascal lets
/// a program set the fields before the tag, or never set it, so every
/// variant's fields are plain fields here; only `file of Fig` and byte views
/// look at the tag (see `file_record!` below).
#[derive(Clone, Copy, Debug, Default)]
struct Fig {
    x: i16,
    y: i16,
    kind: Shape,
    r: f64,
    w: i16,
    h: i16,
}

// In TP the variants shared the 6 bytes after the tag (the largest variant,
// `Circle`'s `Real`); a file holds the variant `kind` selects, zero-padded.
file_record!(Fig { x, y, kind; case kind: 6 { Shape::Circle => [r], Shape::Rect => [w, h] } });

/// Pascal: `Pun = record ... end`
///
/// A variant part with no tag field: TP programs use one to read the same
/// bytes as different types (`p.L := x` then `p.Lo` is its low word). So the
/// record keeps the 4 bytes, in TP's layout, and each field reads or writes
/// its slice of them.
#[derive(Clone, Copy, Debug, Default)]
struct Pun {
    variants: VariantPart<4>,
}

impl Pun {
    // `L` is bytes 0..4
    fn l(&self) -> i32 {
        self.variants.get(0)
    }
    fn set_l(&mut self, l: i32) {
        self.variants.set(0, l)
    }
    fn l_mut(&mut self) -> VariantField<'_, i32> {
        self.variants.field(0)
    }
    // `Lo` is bytes 0..2, `Hi` bytes 2..4
    fn lo(&self) -> u16 { self.variants.get(0) }
    fn set_lo(&mut self, lo: u16) { self.variants.set(0, lo) }
    fn lo_mut(&mut self) -> VariantField<'_, u16> { self.variants.field(0) }
    fn hi(&self) -> u16 { self.variants.get(2) }
    ...
    // `B` is bytes 0..4
    fn b(&self) -> PArray<u8, 1, 4> { self.variants.get(0) }
    ...
}

file_record!(Pun { variants: raw 4 });

fn pascal_main() {
    ...
    f.kind = Shape::Rect;
    f.w = 3;
    f.h = 4;
    pwriteln!(f.w * f.h, " ", 11);
    fl.write(&f);
    p.set_l(0x12345678);
    pwriteln!(p.lo(), " ", i16::from(p.b()[1]));
    // `Inc(p.Hi)` changes the field in place: `hi_mut()` decodes the word,
    // and writes it back into the bytes when the guard is dropped, at the
    // end of the statement (the same trick as the `Bytes` of an untyped
    // parameter).
    *p.hi_mut() += 1;
    p.b_mut()[4] = 0;
}
```

(The accessors are written one per line by codegen; rustfmt spreads them.
A `string[n]` field uses `get_string(offset, n)`, `set_string`,
`string_field`, since a `PString` doesn't know its capacity, as in
`file_record!`'s `string 30`.)

### Runtime

In `bytes.rs` (or a new `variants.rs`):

- `VariantPart<const N: usize>([u8; N])`: `Copy`, `Default`, `Debug`;
  `get<T: FileRecord>(&self, offset) -> T` decodes from `offset` (the
  decoders already accept any bytes: a string's length byte is clamped, an
  enum past its last value reads as the last); `set<T: FileRecord>(&mut
  self, offset, value)` encodes over its slice; `field<T>(&mut self,
  offset) -> VariantField<'_, T>`; and the `_string` forms taking a
  capacity. Offsets are computed by sema, so they're always inside N.
- `VariantField<'a, T>`: the decoded value, `Deref`/`DerefMut` to it, and
  `Drop` that encodes it back. Same shape as `Bytes`.
- `FileRecord for VariantPart<N>`: the bytes as they are (`raw N` in the
  macro).
- `file_record!` gains `; case tag: size { pattern => [fields], ... }`,
  with a `_ => {}` arm for tag values no variant covers (TP leaves those
  bytes alone; we write zeros). Patterns are what `case` codegen already
  emits for labels (`Shape::Circle`, `true`, `b'A'`, `1`).

### Sema

- `RecordType` gets `variant_part: Option<VariantPart>`: the tag (name
  and type, optional), the variants (labels as `ConstVal`s, and each one's
  field list, recursively), the area size, and `repr: Flattened | Bytes`.
  `fields` keeps listing every field, so lookup, `with`, typed constants
  and `field_path` work unchanged; a `Field` in a byte-backed part carries
  `offset` (and, for a string, its capacity) so codegen knows which
  accessor to emit.
- `tp_size(record)` = fixed fields + tag + max over variants, recursively;
  `SizeOf` folds as today.
- Checks: labels are constants of the tag type, distinct, every label's
  variant is listed once (as `case` labels are checked now); duplicate
  field names across variants; a typed constant of a variant record names
  fields of one variant only and, with a tag, gives the tag's value (TP's
  rule; FPC enforces it too); a tagless flattened record (one with a
  pointer in a variant) used as a `file of` element, an untyped argument,
  or a `Move`/`FillChar` operand is an error.
- Lvalues: a byte-backed field as a `var` argument is passed as `&mut
  *p.lo_mut()`, a guard borrowing the whole variant part, so two such
  fields of the same record in one call are refused by the existing
  overlap check (the way `a[1], a[2]` is: the `VarArg` gets `indexed`).
  `Inc`, `Dec`, `:=` on an element or a nested field (`p.B[4] := 0`,
  `n.D.F := -1`) go through `_mut()`; whole-field assignment through
  `set_`.
- `with p do L := 5` is rewritten to `p.L := 5` as now; nothing to add.

### What a program would notice

- Reading a tagged record's inactive variant (above).
- A `Real` in a byte-backed variant round-trips through TP's 6-byte real48,
  so it keeps TP's precision, not an `f64`'s (`p.R := 0.1` then `p.R` is
  what TP had). That's faithful for `Real` and the same approximation
  files already make for `Double`.
- Two byte-backed fields of one record as `var` arguments in one call:
  refused.

## Proposal 2: procedural types

**Runtime type.** A procedural type becomes a Rust function pointer that
may be nil:

```rust
/// A Turbo Pascal procedural variable: a routine or `nil`. `F` is the
/// function-pointer type, so a `ProcVar` is `Copy` and calling it is an
/// ordinary call. `@F = @Square` compares routines by name.
#[derive(Clone, Copy)]
pub struct ProcVar<F: Copy> { routine: Option<F>, name: &'static str }

impl<F: Copy> ProcVar<F> {
    pub const fn nil() -> Self;
    pub const fn of(routine: F, name: &'static str) -> Self;   // `F := Square`
    pub fn is_nil(&self) -> bool;                               // `@F = nil`, `Assigned(F)`
    pub fn get(&self) -> F;         // the routine to call: runtime error 216 for nil
    pub fn address(&self) -> AnyPtr;   // `@F` as a `Pointer`: `AnyPtr::Code(name)` or `Nil`
}
// Default is nil; PartialEq compares names (nil equals nil); FileRecord
// stores 4 zero bytes and reads back nil, as `Ptr` does.
```

Comparison is by name rather than by `fn` address because Rust warns that
function-pointer comparison is unpredictable (identical functions may be
merged), and because `@Square` is already `AnyPtr::code("Square")` for
`SetIntVec`: `@F` as a `Pointer` is the same thing.

**The lifting problem.** `Square` and `Scaled` stored in the same `Func`
variable must have one Rust signature, but lifting gives `Scaled` a
`scale: &mut f64` parameter that `Square` doesn't need. The answer is the
one virtual methods use: all routines that are ever taken as a value of
one *signature class* (TP's compatibility: same parameter modes and
types, same result; different type names with the same signature are one
class, since a `MathFunc` value may be assigned to a `Func`) get the union
of their globals, and so does every routine that calls through a value
of that class. Sema gives each class a pseudo `ProcInfo` (no body, the
type's parameters as synthesized parameter symbols, `name` = the type's):
a member routine "calls" the class and the class "calls" every member,
and a call through a value (`F(x)`, `R.F(x)`, `Handlers[i](x)`) is a call
to the class. `lift`'s fixed point then does the rest, unchanged, and
`check_aliasing` and `find_param_aliases` see the class as the callee
(so `P(count)` with a `var` parameter, where a member uses `count` as a
global, is the existing "passed by `var` to a routine that also uses it"
error, naming the class). Codegen reuses `call_args(class, ...)` for the
captured globals and `signature_params(class)` for the alias; the callee
expression is `place.get()` instead of a name. A member called directly
(`Square(3.0)`) gets the class's globals too, which its doc comment says.

Only globals can be involved: TP forbids nested routines as values, so no
outer local is ever captured this way.

### Generated code

```pascal
type Func = function(X: Real): Real;
var Scale: Real; F: Func;
function Square(X: Real): Real; begin Square := X * X end;
function Scaled(X: Real): Real; begin Scaled := Scale * X end;
procedure Plot(G: Func);
var i: Integer;
begin for i := 1 to 3 do writeln(G(i):0:1) end;
begin
  Scale := 10;
  F := Square;
  writeln(F(2.0):0:1);
  Plot(Scaled);
  if @F = @Square then writeln('square');
  F := nil;
  if not Assigned(F) then writeln('nil');
end.
```

```rust
/// Pascal: `Func = function(X: Real): Real;`
///
/// A procedural type is a function pointer that may be nil. Every routine
/// kept in a `Func` shares this one Rust signature, so the globals any of
/// them use (`scale`, used by `Scaled`) are parameters of them all.
type Func = ProcVar<fn(&mut f64, f64) -> f64>;

/// Pascal: `function Square(X: Real): Real;`
///
/// Stored in a `Func`, so it takes every global a `Func` routine may use.
fn square(scale: &mut f64, x: f64) -> f64 {
    let mut result: f64 = 0.0;
    result = x * x;
    result
}

/// Pascal: `function Scaled(X: Real): Real;`
///
/// Uses globals, which are passed in as parameters.
fn scaled(scale: &mut f64, x: f64) -> f64 { ... *scale * x ... }

/// Pascal: `procedure Plot(G: Func);`
///
/// Calls through a `Func`, so it takes the globals those routines may use.
fn plot(scale: &mut f64, g: Func) {
    for i in 1..=3_i16 {
        pwriteln!(wd(g.get()(scale, f64::from(i)), 0, 1));
    }
}

fn pascal_main() {
    let mut scale: f64 = 0.0;
    let mut f: Func = ProcVar::nil();
    scale = 10.0;
    f = ProcVar::of(square, "Square");
    pwriteln!(wd(f.get()(&mut scale, 2.0), 0, 1));
    plot(&mut scale, ProcVar::of(scaled, "Scaled"));
    if f == ProcVar::of(square, "Square") {
        pwriteln!("square");
    }
    f = ProcVar::nil();
    if f.is_nil() {
        pwriteln!("nil");
    }
}
```

The unused `scale` in `square` is covered by the file's
`#![allow(unused_variables)]`. A `var` parameter in the type is `&mut T`
in the pointer type (`SwapProc = ProcVar<fn(&mut i16, &mut i16)>`; the
elided lifetimes are the usual higher-ranked ones), an untyped one `&mut
dyn ByteView`, an object `var` parameter in a hierarchy with virtual
methods `&mut dyn TShapeMethods`: `signature_params` already spells all
of these. A procedural parameter inside a procedural type (`MaxFunc =
function(A, B: Real; F: MathFunc): Real`) is just `MathFunc` in the
pointer type. An anonymous procedural type (`var F: function(X: Real):
Real`) is spelled out, like an anonymous array.

### Sema

- Parser: `procedure [(params)]` and `function [(params)]: T` as a type
  expression (the lexer already treats `function` after `=`/`:` as a type,
  for `^`); `far`/`near` are ignored already, and `{$F+}` is not required
  (FPC doesn't either; a program that compiled in TP has it anyway).
- `Type::Proc(Rc<ProcType>)`: parameters (mode, type), result, alias, and
  the class's pseudo `ProcId`. `accepts`: `nil`, the same signature
  (modes and types `==`, result `==`), or a routine with that signature.
- A *procedural value context* is where sema looks before treating a name
  as a call: the right side of `:=` to a procedural place, an argument for
  a procedural value parameter, a typed constant of a procedural type, and
  the operand of `@`. There a bare `Name` of a routine is the value (and
  makes the routine a class member), a place of procedural type is copied
  (`Copy`), `nil` is `ProcVar::nil()`. Anywhere else a procedural place is
  a call, as TP says, so `N := F` calls a parameterless `F`, `F = G` calls
  both (an error if they take parameters, as in TP and FPC: the message
  points at `@F = @G`), and `Write(F)` prints the result.
- A call whose callee is a place of procedural type (variable, field,
  element, `p^.F`, a `var` parameter) is checked with `user_call_with(e,
  class, args, None)`, so argument checking, `var` overlap and call sites
  are shared with ordinary calls.
- `@X`: of a routine, `AnyPtr::code` as today (and the routine joins its
  class, so its signature is fixed even if only its address is taken); of
  a procedural place, `f.address()`. `@F = @G`, `@F = @Square`, `@F =
  nil` and `<>` are folded to `f == g`, `f == ProcVar::of(square,
  "Square")`, `f.is_nil()` when both sides are addresses of routines or
  `nil`; mixed with a `Pointer` variable they compare as `AnyPtr`s.
  `Assigned(F)` accepts procedural places.
- Lifting: after `dispatch`, the class pseudo-calls above; `Capture` modes
  unify by the same rule as today (any writer makes it `RefMut`).

### What a program would notice

- `SizeOf(Func)` is 4 and a procedural value in a file or a byte view is 4
  zero bytes, read back as nil, as pointers are.
- Calling a nil procedural variable is runtime error 216 with a clear
  message (TP jumped to address 0: a crash or a hang).

## Alternatives considered

Variant records:

- **Flatten everything** (§11's plan): simple, but punning is *silently
  wrong* (`p.L := x` leaves `p.Lo` at 0), the failure mode `docs/
  architecture.md` §15 calls the worst; and `file of Rec` would have no
  way to pick the variant to write for a tagless record. Sema can't
  detect punning reliably (a program may write both variants legitimately).
- **Bytes for every variant part**: always right, but a pointer (or a
  file, or an object with virtual methods) can't live in bytes, and the
  expression-tree idiom puts pointers in variants; and the common tagged
  case would read `fig.w()` instead of `fig.w` everywhere.
- **Rust `enum`**: needs the tag set before the fields, which Pascal
  doesn't; tagless records have no tag; and reading an inactive variant
  would have to be an error, where TP reads bytes.
- **Rust `union`**: `unsafe` to read, `Copy` fields only, no strings.
- **Flattened plus a hidden "last written variant" marker** to make files
  and byte views right for tagless records: that is byte storage with
  extra steps, and reads of the other variant would still be wrong.
- **`set_x` only, no guard** (`let mut b = p.b(); b[4] = 0; p.set_b(b)`):
  three statements for one, and `var` arguments would need a copy-out
  after the call; the guard is the pattern `Bytes` already established.

Procedural types:

- **Rust closures** (`Box<dyn FnMut>` capturing `&mut scale`): the
  borrow lives as long as the variable, so nothing else could touch the
  global; the same reason nested procedures aren't closures (§15).
- **Globals bundled in a struct passed to every member**: `&mut Globals`
  plus `&mut globals.x` is the double borrow §15 rejects for nested
  procedures; and a second mechanism next to lifting.
- **A trait object per procedural type** (`Box<dyn FuncTrait>` with one
  impl per routine): heavier output for no gain; a `fn` pointer is what a
  Rust programmer would write, and it is `Copy`, which record fields,
  arrays of procedures and `file of` want.
- **Comparing by `fn` address** (`std::ptr::fn_addr_eq`): warned about by
  rustc and unreliable for identical bodies; names cost one `&'static
  str`.
- **`Option<fn(...)>` with no wrapper**: `f.unwrap()(x)` gives a Rust
  panic message instead of a Pascal runtime error, and `@F` as a
  `Pointer` would have nothing to show.

## What would still be refused

Variant records:

- A tagless variant part holding a pointer, file or virtual-method object,
  used as a `file of` element, an untyped argument or a `Move`/`FillChar`
  operand: "`Node` has a variant part without a tag field that holds a
  pointer, so PascRust keeps each variant separately and can't tell which
  one to store as bytes".
- Two fields of one byte-backed variant part passed by `var` in one call:
  "`p.Lo` and `p.Hi` are in the same variant part of `p`, which Rust
  borrows as a whole; copy one into a local first".
- A typed constant naming fields of two variants, or omitting the tag's
  value: "a constant of `Fig` must give `Kind` and the fields of that one
  variant only" (TP's rule).
- `absolute` over a variant record, `Seg`/`Ofs` punning of pointers: as
  before (a stored pointer is 4 zero bytes).

Procedural types (TP's own rules, with TP's reasons in the message):

- A nested routine as a value: "`Inner` is nested inside `Outer`; Turbo
  Pascal doesn't allow a nested routine as a procedural value".
- A method, a standard routine, an `interrupt` procedure: "`Sin` is a
  standard function; Turbo Pascal wants a wrapper: `function FSin(X:
  Real): Real; begin FSin := Sin(X) end`".
- A function returning a procedural type: "Turbo Pascal doesn't allow a
  function to return a procedural type".
- `F = G` where `F` takes parameters: "`F = G` calls both routines, which
  need arguments; to compare the routines themselves write `@F = @G`".
- `@F := P`, `@@F`, `Func(P)` and `Func(P) := F` (a `Pointer` turned into
  a routine): "a `Pointer` can't be turned back into a routine; keep it in
  a variable of a procedural type". (Could be added later: `AnyPtr::Code`
  carrying the `fn` pointer as an `Rc<dyn Any>`, downcast by `Func(P)`.)
- A `var` parameter of procedural type given a routine name: a variable is
  needed, as for any `var` parameter.
- A procedural type declared in a unit whose class members are routines of
  the *program* using globals of a type the program declares: the alias
  in the unit's module couldn't name that type. Refused with a message if
  it ever comes up (it needs a unit to know about its program).

## Questions for the user

1. Representation by shape: a tag field means a flattened tagged union
   (reading an inactive variant doesn't see the other variant's bytes), no
   tag means TP's bytes with accessor methods. Should a tagged record whose
   tag the program *never assigns* (nor fills from a file, a typed constant
   or bytes) count as tagless, so a `case Kind: Byte of 0: (L: LongInt);
   1: (Lo, Hi: Word)` with a decorative tag still puns correctly? It's a
   cheap check in sema; the cost is that a reader sees accessors on a
   record that has a tag. I'd leave it out of the first version and add it
   if a real program needs it.
2. For byte-backed fields, is `p.lo()`, `p.set_lo(v)` and `*p.hi_mut() +=
   1` / `p.b_mut()[4] = 0` acceptable Rust to read, or should every write
   go through `_mut()` (`*p.lo_mut() = v`) to have two accessors instead of
   three?
3. Procedural values compare by routine name, `@F` is the existing
   `AnyPtr::Code`, and `Func(P)` (a `Pointer` cast back to a routine) is
   refused until a program needs it. OK?
4. Not requiring `far`/`{$F+}` on routines used as values (as Free Pascal
   does): fine, or should PascRust warn, since real TP would reject the
   program?

## Build order

Each step is a commit with a translate test (`tests/translate/`) and a run
test (`tests/run/`, `.out` from `tests/gen_expected.sh` where FPC matches
TP), the docs updated (`README.md`, `docs/language_spec.md` tier 2,
`docs/architecture.md` §11 rewritten and §12/§15 extended, `TODO.md`),
and a Fable review after steps 3 and 6.

1. **Tagged variant records, flattened.** Parser and AST for a variant
   part (optional tag, constant-list labels, nested parts), sema's
   `VariantPart` and `tp_size`, the checks on labels and typed constants,
   codegen's struct and the `case` clause of `file_record!`; `with` and
   `var` arguments of variant fields fall out. Tests: `variant_records`
   (a `TPolygon`-style record: assign, read, `with`, pass fields by `var`,
   typed constant, `SizeOf` hand-written since FPC pads the `Real`
   variant, `file of` round trip with an integer variant so FPC's bytes
   match TP's); errors `variant_errors` (a label of the wrong type, a
   repeated label, a typed constant with two variants or no tag).
2. **Tagless variant records as bytes.** `VariantPart<N>`,
   `VariantField`, `raw N` in `file_record!`, the accessors in codegen,
   the place rules in sema, the pointer exception. Tests:
   `variant_punning` (`LongInt` as `Word`s and `Byte`s, a `string[3]`
   over bytes, `Inc` and element writes in place, a record passed to an
   untyped parameter and through `Move`, `file of` round trip,
   `BlockWrite` of the bytes; FPC agrees for all of these; a `Real` case
   with hand-written output), errors: two fields of one part by `var` in
   one call, a tagless record with a pointer in a `file of`.
3. **Nested variant parts and anonymous records in variants**, both
   representations, including a tagless part inside a tagged one. Test:
   `variant_nested` (FPC agrees when no `Real` is involved; `vr2.pas`'s
   `Nested` record gives `65535 255`).
4. **Procedural types: values and calls.** Parser (type expressions,
   anonymous in `var`), `Type::Proc`, signature classes with their pseudo
   `ProcInfo`, value contexts, calls through places, `@` comparisons,
   `Assigned`, typed constants; runtime `ProcVar`; codegen alias and
   calls. Tests: `proc_types` (a `Func` variable, `Plot(Square)`, a
   `SwapProc` with `var` parameters, a procedural field and an array of
   procedures, `nil`/`Assigned`/`@F = @Square`, `N := F` calling a
   parameterless function, a typed constant, `SizeOf(Func)` hand-written);
   errors `proc_type_errors` (nested, method, standard routine, function
   returning a procedural type, `F = G` with parameters, `@F := P`,
   `Func(P)`, a `var` parameter given a routine name).
5. **Lifted globals through procedural values.** The class pseudo-calls
   in `lift`, the doc comments, `check_aliasing` through a class. Tests:
   `proc_globals` (a member using a global, one that doesn't, a caller
   through the value, a direct call to the member, a routine from a unit
   stored in the program's variable, a procedural variable itself global),
   error: a global passed by `var` through a value whose member uses it.
6. **Procedural values as bytes**: `FileRecord for ProcVar`, a record with
   a procedural field in a `file of` and in an untyped parameter (4 zero
   bytes, nil back). Folded into `proc_types` if small.
7. Later, if a program needs it: `Func(P)` via `AnyPtr::Code` carrying the
   routine, and the "tag never assigned" heuristic from question 1.

## The user's answers (2026-10-30)

All four as recommended: (1) a tag field always means a flattened tagged
union; the "tag never assigned" heuristic waits for a real program that
needs it. (2) Three accessors: `p.lo()`, `p.set_lo(v)`, `p.lo_mut()`.
(3) Procedural values compare by name, `@F` is `AnyPtr::Code`, `Func(P)`
is refused for now. (4) `far`/`{$F+}` isn't required, and there's no
warning, as in Free Pascal. Fable reviews are on hold (the user,
2026-10-29), so the reviews after steps 3 and 6 are tracked in TODO.md
instead.
