# PascRust architecture

This document records the core architectural decisions for PascRust, a
Turbo Pascal to Rust transpiler, with the rationale for each and the
alternatives that were rejected. It answers every question in the "Make the
core architectural decisions" section of `TODO.md`. The language subset is in
`language_spec.md`; prior art is in `notes/p2c.md`; things only the user can
decide are in `notes/open_questions.md`.

Guiding priorities, in order (from the opening prompt, in `README.md` as of commit 726b781):

1. **Correct and memory-safe.** Translated programs must behave like the Turbo
   Pascal originals and must never cause undefined behaviour in Rust. Speed is
   explicitly not a goal.
2. **Readable output.** Generated Rust should look like an intermediate Rust
   programmer wrote it, with comments explaining interesting Rust constructs
   and interesting Pascal semantics. The same applies to the compiler itself.
3. **Learning value.** When two options are close, prefer the one that teaches
   more about compilers or Rust.

The examples in `examples/` (about 5,000 lines) are the acceptance test. They
use: units, Turbo Pascal `object` types with static methods and single
inheritance, pointers with `New`/`Dispose`, `file of record` and `Text` files,
nested procedures (recursive, and reading the enclosing procedure's locals),
`var` parameters everywhere, typed-constant arrays, char- and enum-indexed
arrays, and the `Crt`, `Dos` and `Graph` units. They do not use sets, variant
records, `goto`, virtual methods, or procedural types. (Virtual methods are
supported anyway, §16; `with`, `{$I-}`/`IOResult` and `SeekEof` are too,
for other old programs.)

---

## 1. Pipeline overview

```
foo.pas (+ units)                                   out/foo/
   │                                                 ├── Cargo.toml   (depends on pascrust_runtime by path)
   ▼                                                 └── src/
 lexer ──► tokens ──► parser ──► AST ──► sema ──► codegen ──► rustfmt ──►   ├── main.rs
                                          │                                 └── bstunit.rs, founit.rs, ...
                                          └── symbol tables, types,
                                              lifting analysis
```

- **lexer**: handwritten; produces tokens with byte spans. Case-insensitive
  keywords, `{ }` / `(* *)` comments, `{$...}` directives (parsed, mostly
  ignored), the `1..5` vs `1.5` number ambiguity, `#13` and `'it''s'` string
  literals.
- **parser**: handwritten recursive descent, producing a plain `enum`/`struct`
  AST with spans.
- **sema** (semantic analysis): name resolution (units, `with`, objects,
  nested scopes, Pascal's case-insensitivity), type checking of every
  expression, constant folding, and two analyses codegen depends on:
  *lambda lifting* (which outer variables each nested procedure and each
  global procedure touches, transitively) and *lvalue classification* (is this
  `var` argument a local, a field of a local, a heap cell, or a global?).
  Produces a typed AST: the original AST annotated with resolved symbols and
  types. This pass is where most of Pascal's semantics live, and it is what
  lets codegen stay a simple tree walk.
- **codegen**: a tree walk over the typed AST that writes Rust source to a
  string through an indenting writer. Output is then run through `rustfmt`
  (if available) so our formatter does not need to be perfect.
- **driver**: writes a Cargo project, optionally runs `cargo build` /
  `cargo run`. The CLI is `pascrust build foo.pas`, `pascrust run foo.pas`,
  `pascrust emit foo.pas` (print the Rust), and `pascrust fmt`. A second
  binary, `tpc` (`src/bin/tpc.rs`), takes Turbo Pascal's `TPC.EXE` command
  line: it builds in a cache directory and copies the executable next to
  the source; `-U` adds unit directories and `-$` switches act as
  directives at the start of every file (`driver::Options`). We don't
  provide an `fpc` look-alike: it would hide the real Free Pascal, which
  `tests/gen_expected.sh` runs. `run` uses `cargo run
  --manifest-path out/foo/Cargo.toml` rather than running cargo inside
  `out/foo`, so the program starts in the user's current directory and
  relative file names and drive mappings mean what they'd expect. The
  driver also works out which source ranges `{$I-}` covers (§18).

Whole-program compilation: the driver reads the program and every unit it
uses (recursively) before emitting anything, so sema can see everything. Each
unit becomes one Rust module file.

## 2. Repository layout (Cargo workspace)

```
pascrust/
├── Cargo.toml                 # [workspace] members = ["crates/*"]
├── crates/
│   ├── pascrust/              # the transpiler: bin + lib
│   │   └── src/{main.rs, lib.rs, lexer.rs, token.rs, parser/, ast.rs, sema/, codegen/, driver.rs, diag.rs}
│   └── pascrust_runtime/      # the library every generated program depends on
│       └── src/{lib.rs, prelude.rs, string.rs, array.rs, set.rs, ptr.rs, ordinal.rs,
│                system.rs, io/{text.rs, typed.rs, fmt.rs}, run.rs, crt.rs, dos.rs, graph/}
├── examples/                  # the user's Turbo Pascal programs (acceptance tests)
├── tests/                     # pascal sources + expected Rust / expected output (see §21)
├── notes/                     # research notes
└── out/                       # generated Cargo projects (git-ignored)
```

`pascrust_runtime` has Cargo features `crt`, `dos`, `graph`. A hello-world
program depends only on the core (no terminal or window crates get built).
Codegen enables exactly the features the `uses` clauses require.

## 3. Dialect: Turbo Pascal 7 syntax, Turbo Pascal 5.5 feature level

Decision: target **Turbo Pascal** (the Borland dialect), not ISO 7185.
Specifically, accept TP 7.0 syntax (the superset) and implement the features
the examples need, which are the TP 5.5 feature set. ISO-only constructs that
TP lacks (conformant arrays, `pack`/`unpack`, procedural parameters in ISO
form) are out of scope. `language_spec.md` has the tiers.

Rationale: every example starts with `uses Crt` and relies on Borland strings,
units, objects and typed files. ISO Pascal has none of those. The TP 7.0
Language Guide is the de-facto spec and is freely available (links in
`notes/p2c.md`). The sources we checked TP's behaviour against (a disassembly
of the System unit, Free Pascal's scanner, ...) are listed under References
in `language_spec.md`.

## 4. Lexer and parser: handwritten recursive descent

Decision: handwrite both. No parser-generator or combinator crate.

Rationale:
- Pascal was designed for recursive descent; it is LL(1) apart from a few
  one-token lookaheads (`a := x` vs `a(x)`, `1..5` in the lexer, `(` starting
  a typecast vs an expression). A handwritten parser is the natural fit and is
  the most instructive thing to write for someone who has never built a
  compiler.
- Error messages: handwritten parsers give "expected `;` after statement at
  line 42" for free; combinator and PEG errors are notoriously poor.
- Case-insensitivity, nested directive comments, and the `1..5` lexing
  wrinkle are all trivial by hand and all awkward in lexer DSLs. So is `^`:
  `^M` is the character #13, but `var p: ^T` is a pointer to `T`. Like Free
  Pascal's scanner, the lexer reads `^` as the pointer symbol where a type
  can appear (it tracks whether it's in a `type` or `var` section, or a
  typed constant's type) and after something that can be dereferenced.
- Zero build-time machinery and nothing extra to learn.

Rejected:
- **logos** (lexer DSL): fine, but our lexer is ~300 lines either way, and the
  `1..5` vs `1.5` case needs a callback anyway.
- **chumsky** (parser combinators): heavy generics, slow compile times, and
  error types that are hard for a learner to read.
- **lalrpop** (LR generator): a grammar DSL plus a build script; Pascal's
  grammar is not where the difficulty is, and we would lose the hand-rolled
  lookahead tricks.
- **pest** (PEG): yields an untyped parse tree that then has to be converted
  into an AST by hand, so you write the parser twice. (ferropascal, the one
  existing Pascal to Rust project, uses pest and stalls at exactly that stage;
  see `notes/p2c.md`.)

Diagnostics: a small `diag.rs` with `Span` and an error type that prints
`file:line:col: message` plus the source line with a caret. Use the `ariadne`
or `codespan-reporting` crate only if the hand-rolled version proves limiting.

## 5. The memory model and the "virtual heap" idea

The user proposed a virtual heap: a single object representing Pascal memory,
passed around, perhaps memory-mapped, perhaps with an embedded Pascal engine,
raising errors to the Rust host. Here is what we keep and what we drop.

Decision:

- **Scalars, records, arrays, strings and sets are plain Rust values** living
  in Rust locals and parameters. No virtual memory for them.
- **The Pascal heap (`^T`, `New`, `Dispose`) is virtual**, but it is
  distributed rather than one big object: a Pascal pointer is a *handle*
  (`Ptr<T>`, a thin wrapper around `Option<Rc<RefCell<T>>>`), never a Rust
  reference or raw pointer. Dereferencing a nil or disposed handle is a
  checked runtime error, not undefined behaviour. See §12.
- **Runtime errors surface as panics**, caught once at the top by
  `pascrust_runtime::run`. See §17.

Why not one heap object passed around: that is exactly what fights the borrow
checker. If every procedure took `heap: &mut Heap`, then passing
`heap[p].next` as a `var` argument alongside `heap` itself is a double mutable
borrow and does not compile. Rust's native answer to "many owners, mutate
through any of them, check at runtime" is `Rc<RefCell<T>>`, and that is the
idiom an intermediate Rust programmer would reach for. It is the virtual heap
idea, in Rust clothing.

Why not a byte-array heap with every variable access compiled to
`heap.load_i16(addr)`: it would be correct and safe, but the generated code
would be unreadable, which violates priority 2, and it throws away everything
Rust's type system could check for us.

Why not an mmap'd file: it only buys address stability and persistence,
neither of which we need. Pascal programs never observe addresses unless they
use `Seg`/`Ofs`/`Mem`, which are out of scope (`language_spec.md`).

Why not an embedded interpreter ("Pascal engine"): then it is not a
transpiler, and the point of the project is to generate Rust. The one thing
the engine idea gets right, "errors are raised to the Rust container", we keep
via `run()` and `catch_unwind` (§17).

## 6. Type mapping

| Pascal                    | Rust                                   | Notes |
|---------------------------|----------------------------------------|-------|
| `Integer`                 | `i16`                                  | Faithful to TP; see §7 |
| `LongInt` / `ShortInt`    | `i32` / `i8`                           | |
| `Word` / `Byte`           | `u16` / `u8`                           | |
| `Real`                    | `f64`                                  | TP's 6-byte real is a subset of f64 |
| `Single`/`Double`/`Extended`/`Comp` | `f32`/`f64`/`f64`/`i64`      | `{$N+}` types; cheap to add |
| `Boolean`                 | `bool`                                 | |
| `Char`                    | `u8`                                   | TP chars are bytes (CP437); see §9 |
| `String`, `String[n]`     | `PString`                              | 256-byte, Copy; see §8 |
| `array[lo..hi] of T`      | `PArray<T, LO, N>`                     | see §10 |
| `record`                  | `struct` with `#[derive(Clone, Default)]` | see §11 |
| enumerated type           | `enum` + `Ordinal` impl                | see §13 |
| subrange `a..b`           | the host integer type (`i16` etc.)     | checked only with `--range-checks` |
| `set of T`                | `PSet<T>`                              | see §14 |
| `^T`                      | `Ptr<T>`                               | see §12 |
| `Text`                    | `PText`                                | |
| `file of T`               | `PFile<T>`                             | |
| `object`                  | `struct` + `impl`                      | see §16 |
| procedure / function      | `fn`                                   | |
| unit                      | `mod` (one file)                       | |

Everything in the right column except primitive types lives in
`pascrust_runtime` and is re-exported from `pascrust_runtime::prelude`.

## 7. Integers and overflow

Decision: `Integer` is `i16`, and the generated `Cargo.toml` sets
`overflow-checks = false` for all profiles so that `+`, `-`, `*` wrap silently
exactly as Turbo Pascal does. Mixed-width arithmetic gets explicit `as` casts
inserted by codegen following TP's promotion rules (`Integer + LongInt` is
computed in `LongInt`; assignment narrows with truncation).

Rationale: the examples were written against 16-bit `Integer` (`maxint =
32767`, `hi`/`lo` of `WindMax`, `file of record` layouts). Mapping to `i32`
would silently change `MaxInt`, record sizes and wrap-around behaviour.
Disabling overflow checks keeps the arithmetic readable (`a + b`, not
`a.wrapping_add(b)`); this is a generated program, not a library, so the usual
Rust argument for keeping checks on does not apply. Open question #1 offers a
`--integer-width 32` escape hatch.

Division: `/` always produces `f64` (codegen casts both operands); `div` and
`mod` map to Rust `/` and `%`, whose truncating semantics match TP exactly.
Division by zero is a Rust panic, which becomes Pascal runtime error 200
(§17).

## 8. Strings: `PString`, a 256-byte value type

Decision: one runtime type for all Borland short strings, with exactly Turbo
Pascal's memory layout:

```rust
/// A Turbo Pascal string: byte 0 is the length, bytes 1..=255 the
/// characters, copied by value on assignment, exactly like TP.
#[derive(Clone, Copy)]
pub struct PString { bytes: [u8; 256] }
```

- `Copy`, because TP strings are value types and that keeps codegen free of
  `.clone()` noise. 256 bytes per copy is irrelevant at our scale.
- **Capacity is static, not stored.** (Revised by Opus during Phase 1; the
  first draft stored a runtime `cap` byte.) `string[30]` and `string` are the
  same Rust type; codegen knows the declared capacity and emits
  `name = value.truncated(30)` when assigning to a `string[30]` variable
  (skipped for literals that obviously fit). Keeping the capacity out of the
  value means `#[derive(Default)]` works for records containing strings, and
  `s = t` is always plain assignment. The one gap is `var` parameters
  declared `string` receiving a `string[30]` variable under `{$V-}`: the
  callee can store more than 30 characters, which in TP would overwrite
  memory; for us it's simply a longer string.
- 1-based indexing via `impl<I: Ordinal> Index<I>`; `s[0]` reads and writes
  the length byte like TP. Indexes outside 0..255 are runtime error 201.
- Concatenation is `impl<T: Into<PString>> Add<T> for PString`, so
  `s + "abc"` and `s + c` (a `Char`) work directly; comparison is byte-wise
  like TP, with `PartialEq<&str>` so `s == "abc"` reads naturally.
- The `System` string API (`Length`, `Copy`, `Pos`, `Delete`, `Insert`,
  `Str`, `Val`, `Concat`, `UpCase`) is methods (`s.copy(i, n)`,
  `s.delete(i, n)`) or free functions (`pos(sub, s)`, `pas_str(w(x, 5))`).
- Literals: `pstr("hello")` where a `PString` is needed; raw `"hello"` where
  the runtime accepts anything string-like (`pwriteln!`, `+`, `==`).
  Non-ASCII (CP437) literals become byte strings, `b"\xc9\xcd"`.

Rejected: Rust `String`. It is UTF-8, so byte indexing and `s[i] := 'x'` are
wrong or panic on non-ASCII, it has no capacity semantics, and it is not
`Copy`. Also rejected: `PString<const CAP: usize>` with const generics; it is
elegant but infects every runtime function signature with a generic parameter
and makes `string[30]` and `string[40]` different types, which TP does not do.

## 9. Characters and the code page

`Char` is `u8`. Since `Byte` is also `u8`, the runtime's `PasWrite` impl for
`u8` prints a character, and codegen widens `Byte` values (`i16::from(b)`)
before writing them so they print as numbers. Output to the terminal translates CP437 bytes to Unicode (a
256-entry table in the runtime), so box-drawing characters and `chr(7)` work
as they did in DOS. Input (`ReadKey`, `Readln`) translates back where possible
and otherwise yields `?`. `#13`, `#10`, `^M` literals are supported.

## 10. Arrays: `PArray<T, LO, N>`

Decision: a runtime newtype around a fixed-size Rust array with the lower
bound as a const generic parameter:

```rust
/// Pascal `array[LO..LO+N-1] of T`. Pascal arrays can start at any ordinal
/// (`array[1..10]`, `array['A'..'Z']`, `array[Red..Blue]`); Rust arrays
/// always start at 0, so indexing subtracts LO and range-checks.
#[derive(Clone, Copy)]
pub struct PArray<T, const LO: i32, const N: usize>([T; N]);
impl<T, const LO: i32, const N: usize> Index<i32> for PArray<T, LO, N> { ... }
```

- `array[0..maxcol, 0..maxrows] of boolean` becomes
  `PArray<PArray<bool, 0, 22>, 0, 78>` and `grid[c, r]` becomes `grid[c][r]`.
  Codegen converts char and enum indexes with `as i32`.
- `Default` is implemented with `std::array::from_fn(|_| T::default())`, so
  it works for any `N` (the std `[T; N]: Default` impl stops at 32).
- `Copy` when `T: Copy`, so arrays of scalars copy implicitly, matching TP's
  value semantics; arrays of non-Copy elements get `.clone()` inserted by
  codegen.
- Index errors always panic with a TP-style message ("Range check error:
  index 23 not in 0..21"), regardless of `{$R}`; it costs nothing and it is
  the safety guarantee the user asked for.

Rejected: plain `[T; N]` with codegen subtracting the lower bound at every
index (`grid[(c - 0) as usize]`): noisier output and index arithmetic
scattered everywhere. Rejected: `Vec<T>`: wrong value semantics and no size in
the type. Note the const-generic upper bound cannot be derived from `LO` and
`HI` on stable Rust (`generic_const_exprs`), which is why the parameter is `N`
and the transpiler computes it.

Big arrays live wherever the Pascal variable lived; globals are locals of
`main` (§15), so a very large global array could overflow the main thread's
stack. `run()` executes the program on a thread with a large stack (see
§17). Graphics programs do too; the main thread runs their window (§19).

## 11. Records and variant records

Records are structs: `#[derive(Clone, Default, Debug)]`, plus `Copy` when
every field is `Copy`. Field names are snake_cased (§20). Record assignment is
a Rust move/copy or `.clone()`, inserted by codegen based on the type.
`Default` is derived so `var r: Rec;` becomes `let mut r = Rec::default();`
(TP leaves locals uninitialized; zero-initializing is strictly safer).

Variant records (`case tag of ...`) are **flattened**: every field of every
variant becomes an ordinary field of the struct. All normal uses (read and
write the fields of the active variant) work. What breaks is type punning
through overlapping variants and `SizeOf`, which the examples do not do and
which `language_spec.md` lists as unsupported. Rejected: Rust `enum` (requires
the tag to be set before the fields, which Pascal does not require, and free
variants have no tag at all); Rust `union` (unsafe, and `Copy`-only fields).

## 12. Pointers and the heap: `Ptr<T>`

```rust
/// A Turbo Pascal pointer. `Rc` gives shared ownership (several Pascal
/// pointers can point at one record); `RefCell` moves Rust's exclusive-access
/// rule from compile time to run time, which is what lets `p^.next^.prev^`
/// style code compile at all.
pub struct Ptr<T>(Option<Rc<RefCell<T>>>);
```

- `nil` is `Ptr::nil()` (`None`). `Default` is nil.
- `New(p)` → `p.new_()` allocates `T::default()`. `Dispose(p)` → `p.dispose()`
  sets `p` to nil and marks the cell disposed; other handles to the same cell
  panic on access ("use after Dispose") instead of reading freed memory.
  Memory is freed when the last handle drops, which is a safe superset of TP's
  behaviour.
- `p^` → `p.get()` (a `Ref<T>`) for reads, `p.get_mut()` (a `RefMut<T>`) for
  writes. `p^.field := v` → `p.get_mut().field = v;`. `p^ := q^` →
  `*p.get_mut() = q.get().clone();`.
- `p = q` compares identity (`Rc::ptr_eq`), as in Pascal.
- Codegen hoists any right-hand side that dereferences a pointer into a `let`
  temporary before an assignment or a call, so no `Ref` guard is ever alive
  while the same cell might be borrowed mutably (e.g. `p := p^.next` becomes
  `let next = p.get().next.clone(); p = next;`). This is the one codegen rule
  that keeps `RefCell` panics away from ordinary list and tree code; the
  Rust-level reason (a temporary borrow living until the end of the statement)
  gets a comment in the output.
- Untyped `Pointer`, `GetMem`/`FreeMem`, pointer arithmetic, `Addr`/`@` on
  variables, `Seg`/`Ofs`/`Ptr(...)` and `Mem[]` are out of scope.

Run-time aliasing hazard: a `var` argument that is a heap field, e.g.
`Remove(p^.next)`, is passed as `&mut p.get_mut().next`, which keeps `p`'s cell
mutably borrowed for the whole call. That is fine for tree and singly-linked
code (the examples' `RecInsert(root^.before, ...)` never touches `root` again
through another path), but a doubly-linked list that reaches `p` through
`node^.prev` during the call will hit a `RefCell` "already borrowed" panic.
This is a loud failure, never memory unsafety, and the runtime wraps it in a
clear message. It is the accepted cost of readable output; the fully general
alternative (passing every `var` parameter as a "place" descriptor that
re-borrows on each access) would make every `var` parameter use in every
procedure body look like `x.set(x.get() + 1)`. If a real program hits it, the
fallback is a transpiler flag that switches that one procedure's `var`
parameters to the place representation. Tracked in `TODO.md` (tier 3).

## 13. Enumerations, subranges and the `Ordinal` trait

Enumerated types become `enum`s with
`#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]`
(`#[default]` on the first variant) and generated impls of the runtime's
two traits:

```rust
pub trait Ordinal: Copy { fn ord(self) -> i64; }
pub trait Enumerated: Ordinal + 'static { const VALUES: &'static [Self]; /* from_ord, ... */ }
```

`Ordinal` turns a value into its position, and `u8` (char), `bool` and the
integer types implement it too; `Enumerated` lists an enum's values in
order, which is all that's needed to go back (`Color(2)`, `Succ`, `Pred`,
`for x := Lo to Hi`). Enum-indexed arrays go through `Ordinal`.

Subranges (`0..maxsize`, `'A'..'Z'`) are their host type; a `--range-checks`
flag makes codegen wrap assignments into subrange variables with
`check_range(v, lo, hi)`. Off by default because TP's default is `{$R-}`.

## 14. Sets

`set of T` becomes `PSet<T>`, a 256-bit bitset (`[u64; 4]`, `Copy`, like
TP's 32-byte sets) with a phantom element type. `+`, `*` and `-` are
`|`, `&` and `-` (`BitOr`/`BitAnd`/`Sub`); `=`/`<>` compare the bits;
`<=`/`>=` are `is_subset`/`is_superset`; `Include`/`Exclude` are methods.
`s := s + [x]` is `s |= pset![x]` (and `&=`, `-=`), except when the
right side reads the same variable or calls anything: `s[i] |= s[1]` would
borrow `s` for the place first (the operator is a trait method), and a
call may be passed the variable without naming it (a global the function
uses, or `self`), so those stay `s[i] = s[i] | s[1]`. A backwards range (`[Fri..Mon]`) is empty, as in TP.
The element methods take any `Ordinal`, so `300 in s` is simply false
rather than wrapping to a byte. Set literals `[1, 3..5, x]` become
`pset![1, 3..=5, x]` (each item is a value or a `RangeInclusive`, through
the `SetItems` trait), and `x in [...]` with only constants in the list
becomes `matches!(x, 1 | 3..=5)`, which is what a Rust programmer would
write (an enum range lists its values there: Rust range patterns are for
numbers and chars). With other items, `x in [Mon..Wed, y]` tests a set
built on the spot, and as nothing else says what that set holds, the
literal says it: `pset![set of Day; Day::Mon..=Day::Wed, y].contains(x)`. Set constants are Rust `const`s, built by the `const fn`s
`PSet::of(&[...])` and `PSet::from_ranges(&[...])` from `u8` ordinals,
since trait methods can't run in a `const`. The element type must have
ordinals 0 to 255 (`Char`, `Byte`, `Boolean`, an enum, a subrange in that
range), as in TP. p2c's small-set/big-set split is unnecessary here; one
representation, no optimization.

## 15. Procedures, parameters and the aliasing problem

### Value parameters

Pascal copies value parameters; the callee may mutate its copy. Codegen passes
by value (`grid: GridT`) and inserts `.clone()` at the call site for non-Copy
types. The parameter is declared `mut` only if the body assigns to it.

### `var` parameters: `&mut T`

Decision: a `var` parameter is a `&mut T` parameter, and a `var` argument is
`&mut lvalue`. This is the readable, idiomatic choice and it is correct in all
of the common cases:

- Locals and fields/elements of locals: `setup(&mut grid, &mut generations)`.
- Disjoint fields of one struct in the same call: Rust allows
  `init(&mut self.center, &mut self.radius)` because the borrows are disjoint.
- Reborrowing a `var` parameter to pass it on: `rec_insert(root, item, ok)`
  (Rust reborrows `&mut` implicitly).
- Heap fields: `&mut p.get_mut().before` (see the hazard note in §12). Two
  fields of the same heap record, `SetTwo(q^.x, q^.y)`, would be two
  `get_mut()` borrows of one `RefCell`, so the call borrows the record once:
  `q.with_mut(|point| set_two(&mut point.x, &mut point.y))`. The pointer
  may be indexed by a variable or a number (`arr[i]^.x, arr[i]^.y`), and
  a path behind it stops at an array index, as for ordinary variables.

Sema compares field paths, so only arguments that really overlap are
rejected: `p.x` and `p.y` are fine, `p` and `p.x` are not. A path stops at an
array index, because Rust borrows the whole array to index it: `a[1]` and
`a[2]` count as the same place.

Where Rust rejects a call and Pascal accepts it, the cause is always
*aliasing*: the same storage reachable through two parameters at once. The
three cases and what we do:

1. **The same variable passed twice** (`Swap(x, x)`) or **a variable and a
   part of it** (`F(rec, rec.field)`): sema detects the overlapping lvalue
   roots and reports a transpile error naming the Pascal line. These are bugs
   or near-bugs in the Pascal and are vanishingly rare in practice.
2. **A global passed by `var` to a procedure that also uses that global
   directly**: because globals are lifted into parameters (§15.4), this is
   case 1 again and sema reports it the same way.
3. **Heap aliasing** (`Remove(p^.next)` where the callee reaches `p` through
   another pointer): undetectable statically; becomes a runtime `RefCell`
   panic with a clear message (§12).

Rejected alternatives for `var`:
- **Copy-in/copy-out** (pass a temporary, write it back after the call):
  semantically exact only without aliasing, and *silently wrong* with it,
  which is the worst failure mode. Reserved as a per-call-site strategy sema
  may use for a global whose callee provably never touches it.
- **A `Place` trait** (`get`/`set`/`update` descriptors): always correct, but
  every `var` parameter use in every body becomes `x.set(x.get() + 1)`,
  generics appear on every procedure, and the output stops looking like Rust.
  Kept as a tier-3 opt-in per procedure.
- **Raw pointers**: aliasing `&mut` through raw pointers is undefined
  behaviour even when "it works"; it would void the whole safety premise.

### Function results and `Exit`

A function body gets `let mut result: T = Default::default();` named after the
function; `F := expr` assigns to it; the body ends with `result`; `Exit`
becomes `return result;` (or `return;` in procedures). The function name on the
right-hand side inside its own body is a recursive call (TP rule); sema
distinguishes the two by position.

### Globals: lifted into `main`

Decision: a program's global variables (and each unit's interface and
implementation variables) become locals of `fn main`, and every procedure
that touches a global, directly or through anything it calls, receives it as
an extra parameter (`&mut T`, or `&T` / by value for read-only scalars). This
is the same lambda-lifting mechanism used for nested procedures (§15.5), so
there is only one mechanism to understand, and `main` reads as "here are all
my variables; here is my program".

Rationale: a `static mut` would need `unsafe` on every access; a
`thread_local!` `RefCell` would turn every access into a closure
(`GRID.with_borrow_mut(|g| g[c][r] = v)`) and would panic when a global is
passed by `var` to a procedure that reads any global through the cell. Lifting
makes global access look like local access (`grid[c][r] = v`) and turns the
aliasing problem into an ordinary compile-time/sema check (case 2 above). The
examples are written school-style (everything passed as parameters, globals
rarely touched inside procedures), so lifted signatures stay short. For
programs where a procedure touches dozens of globals, the signature will be
long; that is honest, and a tier-3 option can bundle a unit's globals into a
`struct Globals` passed as one `&mut` parameter when no `var` argument comes
from that struct.

Unit initialization sections become `pub fn init(...)` taking the unit's
globals, called from `main` in `uses` order before the program body.

### Nested procedures: lambda lifting

Decision: every nested procedure becomes a top-level `fn` named
`outer_inner` (`report_visit`), with one extra parameter per enclosing-scope
variable it uses, transitively (a nested procedure that calls a sibling which
uses `total` also needs `total`). Read-only `Copy` captures are passed by
value; everything else by `&mut`. The call site in the outer procedure passes
`&mut total`. Recursion works because these are ordinary functions.

Rejected: **Rust closures**. A closure capturing `total` by `&mut` cannot be
called recursively (a closure cannot name itself), and its borrow of `total`
is live from creation to last use, so the outer body cannot touch `total` in
between. Rejected: **p2c's static-link struct** (`struct LOC_outer { total,
... }` passed by pointer): the natural C solution, but in Rust `&mut LOC` plus
`&mut LOC.total` is a double borrow, so it reintroduces the aliasing problem
for no gain. Per-variable lifting borrows disjoint variables, which Rust
accepts. (See `notes/p2c.md` for Gillespie's description of the struct scheme.)

Nested procedures inside object methods that use `Self` fields (the examples'
`Winner` inside `BoardType.Finish`) receive `this: &mut BoardType`.

## 16. Objects (Turbo Pascal 5.5 OOP)

The examples use `object` types heavily, always with static methods. Virtual
methods are supported too (below).

- `TypeName = object ... end` → `struct TypeName { fields }` plus
  `impl TypeName { fn method(&mut self, ...) }`. (Methods that never write
  `Self` fields could take `&self`; for now they all take `&mut self`, see
  TODO.md.) Implicit field access (`root := nil` inside
  `BST.Create`) becomes `self.root = Ptr::nil()`.
- Inheritance `Child = object(Parent)` → composition:
  `struct Child { base: Parent, own_fields }`. Inherited field and method
  access resolve to `self.base.x` / `self.base.m()`; passing a `Child` where a
  `Parent` is expected passes `&mut child.base` (or `child.base.clone()`).
  Sema knows the hierarchy, so the rewriting is mechanical. (By value, the
  copy is the ancestor part only. TP copied the hidden virtual method
  table pointer along with the fields, so its copy kept calling the
  descendant's virtual methods on fields it didn't have; ours acts as the
  ancestor it now is.)
- Objects declared as variables and never "created" (`var Board: BoardType;
  Board.Create;`) are `let mut board = BoardType::default();` followed by
  `board.create();`, which is exactly what TP does.
- **Virtual methods** (the design and its reasoning are in
  `notes/virtual_methods.md`). Each object in a hierarchy with a virtual
  method gets a trait, `TCircleMethods: TShapeMethods`, holding an
  accessor for its part of the object (`fn t_circle(&mut self) -> &mut
  TCircle`), the virtual methods it introduces, and, as default methods,
  its method bodies that must see the real object: the virtual versions
  (`t_circle_draw`) and static methods that call a virtual one (sema's
  `dispatched` fixed point). In a default method `self` is whatever object
  the method was called on, so `self.draw()` runs that object's version.
  Every object implements its ancestors' traits, choosing the nearest
  version of each virtual method. Plain variables use static dispatch; a
  `var` parameter of the base type is `&mut dyn TShapeMethods`; every
  pointer type into the hierarchy is `Ptr<AnyShape>`, an enum with a variant
  per object, which implements every member's trait by `match`ing (a
  descendant's accessor and methods are a runtime error on a variant that
  isn't that object). So a method first declared below the root works
  through a pointer (`pc^.Report`), and passing `p^` to a `var` parameter
  of an object type passes the whole enum, `&mut *p.get_mut()`, keeping
  the record's real type. All versions
  of a virtual method receive the same lifted globals, since they share the
  trait's signature. Constructors and destructors are ordinary methods: the
  enum variant, set when the record is allocated, plays the role of TP's
  hidden table pointer. `TypeOf(x) = TypeOf(TCircle)` compares type names:
  when a program uses `TypeOf`, the root trait gets `fn type_name(&self)
  -> &'static str`, and a method using `TypeOf(Self)` becomes a default
  method so it sees the real object.
- `private` sections are a no-op.

Rejected: a `Deref` to `base` trick to make inherited fields look flat. It
hides what is going on, and intermediate Rust programmers are warned off
`Deref` for inheritance.

## 17. Runtime errors: panic, caught once at the top

Decision: every Pascal runtime error (range check, nil/disposed pointer, I/O
error with `{$I+}`, division by zero, `RunError`, `RefCell` double borrow) is a
`panic!` raised through `pascrust_runtime::rte(code, msg)`. The generated
`main` is:

```rust
fn main() {
    // run() runs the program on a worker thread; when it ends or panics,
    // run() restores the terminal (raw mode, colours), saves the graphics
    // screen if asked to, prints "Runtime error 201 ..." in TP style, and
    // exits with the error code.
    pascrust_runtime::run(|| pascal_main());
}
```

`run` joins the worker thread, which returns the panic payload, so cleanup
always happens. (The cleanup runs on the main thread, which is why the "is
the terminal raw?" flag is a process-wide atomic rather than part of Crt's
`thread_local!` state.) `Halt(n)`
flushes output and calls `process::exit(n)`.

Rationale: `Result` would put `?` on every array index, pointer dereference
and `writeln`, destroying readability, and Pascal's own semantics are "abort
the program with a code", which is what a panic is. For embedding (§19) the
same `run` returns `Result<(), PascalError>` to the host, so the host sees an
error value, never a panic, exactly the "exception raised to the Rust
container" the user described.

Programs run on a spawned thread with a 256 MB stack so deep recursion and
large global arrays do not overflow. With the `graph` feature, the main
thread, which macOS requires for window event handling, runs the graphics
window meanwhile (§19).

## 18. Units, `with`, control flow, I/O

- **Units** → one `pub mod` file each. Interface declarations are `pub`;
  implementation-only ones are private. `uses A, B` → `use crate::a::*;
  use crate::b::*;`; sema resolves conflicts by Pascal's rule (the later unit
  wins) and emits qualified paths when needed.
- **`with r do`** → sema rewrites the body, writing each field (or method)
  the record supplies out in full (`r.x`, `pts[i].y`), and checks and emits
  the rewritten statement (`Analysis::with_bodies`). No reference to the
  record is held, so no borrow outlives a statement. TP finds the record
  once, before the body runs; rewriting finds it at every use, which only
  differs if the body changes what chooses it:
  - Through a pointer, `with p^ do`, the pointer is copied first
    (`node = p.clone();`, which copies the `Rc` handle) when the body
    mentions `p` or the pointer is a longer path (`head^.next^`). So
    `with p^ do begin ...; p := next end` keeps meaning the first record.
  - An array index the body changes (`with a[i] do ... i := 2`) is an
    error. (A procedure the body calls that changes `i` as a global isn't
    caught.)

  (p2c takes a temporary pointer to the record; with Rust's borrow checker
  that reference would block every other use of the variable in the body.)
- **`for i := a to b`** → `for i in a..=b` (bounds evaluated once, empty if
  `b < a`, exactly TP); `downto` → `(a..=b).rev()`; enum loops go through
  `Ordinal`. **`repeat ... until c`** → `loop { ...; if c { break } }`.
  **`case`** → `match` with `|` alternatives, `..=` ranges (an enum range,
  `Mon..Fri`, lists its values: Rust ranges are for numbers and chars),
  labels an earlier arm covers dropped with a comment, or trimmed to the
  values no earlier label has (Rust warns about a pattern that can't
  match), and `_ => {}` when
  there is no `else` (none when the labels cover every value, as they can
  for an enum or `Boolean`; an `else` then can't run, and is kept under
  `#[allow(unreachable_patterns)]` with a comment saying so).
  `Break`/`Continue` map directly, except `Continue` in
  a `repeat`, which must still run the `until` test: the body goes in a
  labeled block, `'repeat_body: { ... }`, and `Continue` is
  `break 'repeat_body;`. `goto` is unsupported.
- **System variables** (`ExitCode`, `RandSeed`, `FileMode`, and a unit's,
  like Crt's `TextAttr`, listed in `units::unit_vars`) live in the
  runtime, as a getter and a setter (`SymKind::RuntimeVar`): reading is
  `rand_seed()`, assigning is `set_rand_seed(42)`. Only plain reads and
  `:=` are allowed, not `var` arguments or `Inc`. `ExitCode` and
  `FileMode` are atomics, since the main thread reads `ExitCode` when the
  program's thread ends. `Input`/`Output` (`SymKind::StdText`) as a file
  argument just mean the console: `Writeln(Output, x)` is `pwriteln!(x)`.
  Passed to a `var f: Text` parameter, they're `&mut PText::console()`
  (and `Lst` is `&mut PText::printer()`): a fresh `Text` value already
  open on the console or printer, which shares the real one's state.
- **`and`/`or`** → `&&`/`||` (TP defaults to short-circuit, `{$B-}`); on
  integers they are `&`/`|`. `xor` and `shl` map directly, with two TP
  details: `shr` is a logical shift (`-300 shr 4` is 4077), so a signed
  value is shifted as unsigned; and shifting by the type's width or more
  gives 0, so unless the count is a literal below the width the shift is
  `x.checked_shl(n as u32).unwrap_or(0)`. A shift has the type of its left
  operand.
- **`Write`/`Writeln`** → `pwrite!(...)` / `pwriteln!(...)` macros from the
  runtime that accept any `PasWrite` value and the width helpers `w(x, 8)` and
  `wd(x, 8, 2)` for `x:8` and `x:8:2`; `writeln(f, ...)` → `pwriteln!(f; ...)`.
  Booleans print `TRUE`/`FALSE`, reals default to TP's `1.0000000000E+00`
  form, so output matches TP byte for byte. Output goes through the runtime
  so that `Crt` can track the cursor. Each value is written before the next
  is worked out, as TP defines `Write(a, b)` to be `Write(a); Write(b)`: a
  function in the list that writes something comes out in its place.
- **Evaluation order** isn't defined by Pascal. Where it shows (a function
  that changes a variable the expression also reads), PascRust follows
  Free Pascal: `n := n + Bump` reads `n` first, so when a call could change
  `n` (it's passed `n`, receives it as a global or outer variable, or is a
  method) it stays `n = n + bump(&mut n)` rather than `n += ...`, which
  calls first. Otherwise (`total := total + Price(i)`) it's `+=`. (Free Pascal calls first for
  reals, strings and sets, and works out an assignment's target index
  first; PascRust doesn't yet. See TODO.md.)
- **`Read`/`Readln`** → `preadln!(x, y)`, with TP's lenient numeric parsing
  (leading blanks, stops at the first bad character).
- **Files**: `PText` wraps a buffered reader/writer; `PFile<T>` serializes
  records with a generated `impl PascalRecord for T` (little-endian integers,
  length-prefixed fixed-size strings). TP-exact binary layout (6-byte `Real`)
  is tier 3 and an open question, since the examples ship `.dat` files.
  `Assign`, `Reset`, `Rewrite`, `Append`, `Close`, `Seek`, `FilePos`,
  `FileSize`, `Eof`, `Eoln`, `Erase`, `Rename`, `Truncate`, `IOResult` are
  runtime methods. `{$I-}` is a compile-time switch, so the driver works
  out which source ranges it covers, and codegen wraps each I/O call there
  in `io_unchecked(|| f.reset())`. That catches the runtime error panic (for
  codes below 200; others still stop the program) and keeps the code for
  `IOResult`. Not modelled: TP also skips *checked* I/O, and then stops, while
  an unchecked error is still waiting to be read; and TP runs each argument
  of `Write`/`Writeln` as its own call, so in `WriteLn('x', IOResult)` the
  `'x'` is skipped but `IOResult` clears the error and the rest prints. We
  wrap the whole `Writeln`, so all of it is skipped.

## 19. Crt, Dos, Graph, and the units we fake

- **Crt** (`feature = "crt"`): implemented over the `crossterm` crate (raw
  mode, cursor movement, 16 colours, key polling). The runtime keeps TP's
  state: text attribute, `WindMin`/`WindMax` window rectangle, `LastMode`;
  `ClrScr`, `GotoXY`, `WhereX/Y`, `TextColor`, `TextBackground`, `Window`,
  `ClrEol`, `ReadKey` (with the `#0` + scan-code convention for arrows),
  `KeyPressed`, `Delay`, `Sound`/`NoSound` (terminal bell / no-op), `TextMode`.
  When stdout is not a TTY (tests, CI) the runtime drops to plain mode: cursor
  calls become no-ops, `ReadKey` reads bytes from stdin, so interactive
  programs can be driven by a script.
- **Dos** (`feature = "dos"`): `GetDate`/`GetTime` (via `chrono` local time),
  `FExpand`/`FSearch`/`FSplit` with a DOS-path translation layer (`a:\x` →
  a directory from a config map; open question), `Exec` → `std::process::
  Command` (the examples run `turbo.exe`, which will simply fail), `ChDir`,
  `GetEnv`, `DosVersion` stub. `Intr`, `MsDos`, `Keep`, interrupt vectors:
  unsupported.
- **Graph** (`feature = "graph"`): a software-rendered BGI emulation. A
  640x480 framebuffer of colour *numbers* (`Vec<u8>`) plus a 16-entry EGA
  palette, Bresenham lines (styles, thickness, XOR mode), arcs and ellipses
  (parametric, joined with short lines), 8x8 fill patterns, flood fill,
  viewports, and the default 8x8 bitmap font (the public-domain IBM VGA
  glyphs from `font8x8`, ASCII only) for `OutTextXY`. Displayed through
  `minifb` (one dependency, trivial API, works on macOS). Keyboard input for
  `ReadKey`/`KeyPressed` switches to the window's events while graphics mode
  is active, because the DOS programs mix `Crt` input with `Graph` output.
  `InitGraph(Detect, ...)` always succeeds with "VGA 640x480";
  `GetMaxX`/`GetMaxY` return 639/479.

  Threads: the program draws on its worker thread into a `Mutex`-protected
  screen; the main thread (`graph::window::main_loop`, called from `run`)
  opens and closes the window when asked through an atomic state variable,
  copies the screen into it at 60 fps, and queues keys (a `Mutex<VecDeque>`
  plus `Condvar`). Closing the window ends the program.

  Changed from the plan, and why: colour numbers rather than `u32` pixels,
  because `GetPixel` returns colour numbers and `SetBkColor`/`SetPalette`
  recolour what is already on the screen, as on an EGA. No PNG writer:
  `PASCRUST_GRAPH=headless` draws without a window, and run tests check
  drawings with `GetPixel` instead of golden images (no binary files in the
  repository, no image dependency). `PASCRUST_GRAPH_SAVE=file.bmp` saves the
  screen (BMP needs no compression code) for looking at by eye.

Rejected for Graph: `pixels` + `winit` (more code and an event-loop
inversion that fights Pascal's synchronous style), terminal-based pseudo
graphics (loses the point), and stubs (the clock and tic-tac-toe examples are
graphics programs; stubs would make them meaningless).

### Faking what a modern machine doesn't have

Some of TP's standard library exists because of DOS hardware and limits
that are gone: a printer port, 640 KB of memory, a CGA card. The rule for
those is to **fake it**: accept the call and do something harmless and
plausible, so an old program runs unchanged, and say so (a doc comment in
the runtime, the language spec, and a comment in the generated code where
a call becomes nothing). A real implementation is used wherever one makes
sense; only what can't be real is faked.

- **Printer** (`runtime/src/printer.rs`): `Lst` is a standard file like
  `Output` (`SymKind::StdText(StdFile::Printer)`), and `Writeln(Lst, ...)`
  is `fwriteln!(Lst, ...)` with the runtime's `Lst`. Where it prints is the
  `PASCRUST_PRINTER` setting: appended to `printer.txt` where the program
  started (the default) or another file, or `|command` to pipe it to a
  command run by `sh`, such as `|lp` for a real printer. The destination
  opens on the first print, so a program that never prints creates
  nothing, and is closed by `io::shutdown` on every way out of the program,
  waiting for a command to finish so the whole job is sent. It's a
  `Mutex`, since the program prints on its worker thread but may end on
  the main one.
- **Overlay** (`runtime/src/overlay.rs`): TP's overlays loaded units from a
  `.OVR` file into a small buffer on demand, so programs bigger than 640 KB
  could run. A Rust program has all its code in memory, so there's
  nothing to load. `OvrInit` and `OvrInitEMS` set `OvrResult` to `ovrOk`
  (no `.OVR` file is looked for, so the usual `if OvrResult <> ovrOk then
  Halt` check passes); `OvrSetBuf`/`OvrGetBuf` and `OvrSetRetry`/
  `OvrGetRetry` remember and return a size (`OvrGetBuf` starts at 64 KB,
  so `OvrSetBuf(OvrGetBuf + n)` works); `OvrClearBuf` does nothing;
  `OvrLoadCount` and `OvrTrapCount` stay 0. The `{$O+}`, `{$O UnitName}`
  and `{$F+}` directives are ignored silently: they only told the compiler
  what to put in the `.OVR` file. The calls stay ordinary function calls
  in the generated code (`ovr_init("GAME.OVR")`), so a reader sees what
  the program asked for.
- **Dos** (`runtime/src/dos.rs`): `DosVersion` is 6.22; `DiskFree`/
  `DiskSize` report 512 MB free of 1 GB on every drive, because a real
  disk's size overflows TP's `LongInt`; `SetDate`/`SetTime` don't touch
  the computer's clock; the Ctrl-Break and verify switches are remembered.
  `SetFAttr`'s `Hidden`, `SysFile` and `Archive` bits have nowhere to go on
  a modern file system, so they're remembered while the program runs, by
  device and inode on Unix so they follow a renamed file; a changed
  modification time sets `Archive` again, and a changed creation time
  means a new file. `ReadOnly` really changes the permissions (on Unix,
  clearing it gives write access back to the owner only). The routines that take
  TP's untyped `var F` take any file through the runtime's `AnyFile` trait.
  DOS kept a `FindFirst` search's state in the `SearchRec` itself, so a
  program never closes a search; PascRust keeps each search's matches in
  a table, with the handle and position in `SearchRec.Fill`. A copy of a
  `SearchRec` can carry on with a search at any time, so none is dropped
  when it runs out; instead, past 100 open searches the one used longest
  ago goes. (A loop over `*.PAS` that checks whether each `.BAK` exists
  starts a search per file, while the loop's own search stays in use; a
  recursive directory walk leaves each level's search idle while it
  searches below, so up to 10,000 searches are kept.) Searches that have
  run out are rarely used again, but a listing of a big directory is a
  long list, so only the 100 used most recently of those are kept.
- **System and Crt** (`MemAvail`/`MaxAvail` report 512,000 bytes,
  `Mark`/`Release` and `SetTextBuf` become comments, `Test8087` is 3,
  `DirectVideo`/`CheckSnow`/`CheckEOF` are remembered and ignored): see
  `language_spec.md`.
- Not faked, but refused with a clear error: what needs real DOS or raw
  memory (`Intr`, `MsDos`, `Mem`, `Port`, `GetIntVec`, `Keep`). A unit
  routine can be declared `refused` in `units.rs`, so calling it says why
  instead of "unknown identifier".

## 20. Naming and readability of generated code

- Pascal identifiers are case-insensitive; sema canonicalizes to the spelling
  at the declaration, and codegen converts: types and enum variants to
  `CamelCase`, functions/variables/fields to `snake_case`, constants to
  `SCREAMING_SNAKE`. Rust keywords and runtime names get a trailing
  underscore (`type_`, `move_`, `self_`), as do the names the generated
  code uses by itself: runtime types (`PSet_`), helpers the Rust calls
  (`random_real_`, `set_rand_seed_`, `enum_range_`), `Default`, an
  object's methods named after derived traits (`clone_`, `default_`,
  `fmt_`, `encode_`, `decode_`), an object field `base_` (a descendant
  keeps its parent's part in `base`), and types shaped like a hierarchy's
  generated enum and traits (`AnyShape_`, `TShapeMethods_`). The examples declare a procedure
  named `Exit` and a type named `ShortString`; sema's scoping handles both
  (user declarations shadow builtins, as in TP).
- Every generated declaration carries a `///` comment with the original
  Pascal header line.
- The program's own comments are carried across as `//` comments
  (`codegen/comments.rs`). Codegen reorders things (constants to module
  level, locals to the top of the function, nested procedures out), so the
  lexer keeps each comment's position and codegen attaches it to an
  *anchor*: a statement, a declaration, a record field, a procedure, or a
  block's `end`. A comment ending the line an anchor ends on stays at the
  end of that anchor's Rust line (`a: Integer; {pos of ch}` →
  `let mut a: i16 = 0; // pos of ch`); any other comment goes on its own
  lines before the next anchor, wherever that's written. A comment at the
  end of a loop or `if` header (`for ... do {each row}`) becomes the first
  line inside the Rust block, since rustfmt puts comments after a `{` there
  anyway. A comment inside a statement (`b := 2 + {why} 3;`) goes at the
  end of that statement's line. The comment after an `end` goes after the
  `}` that closes the block, except `end else begin {otherwise}`, whose
  comment starts the else branch. Comments before a `case`'s `end` go
  before the `_ => {}` arm (rustfmt would take them for the last arm's), or
  onto the match's closing brace when there is no such arm.
  Comments before or on the `program`/`unit` line, and up to its `uses`
  clause, head the Rust file; one after a `program` line with no `uses`
  goes with the first declaration instead, since it may well be about
  that. Declarations that make no Rust of their own still pass their
  comments on: a `forward`, interface or object-method heading's go with
  its body (matched by name within the same procedure), a loop variable's
  where its `let` would have been (`// Pascal: \`r, c: integer\` // r=row`),
  an alias's (`TInt = Integer`) on a `// Pascal:` line of its own, and a
  unit with no initialization code keeps the comments at its end.
  Attaching is done with a table of line starts and binary searches, so it
  takes a moment even for a long program.
- Codegen emits teaching comments at the first use of each notable construct
  in a file (`Rc<RefCell>`, const generics, lifted parameters, `catch_unwind`),
  not at every use, to avoid noise.
- `rustfmt` runs on the output; codegen only needs to produce syntactically
  valid, roughly indented Rust.

## 21. Testing strategy

Three layers, all driven by `cargo test`:

1. **Unit tests** in the lexer, parser and sema (token streams, AST shapes,
   type errors), using `insta` snapshots for AST dumps.
2. **Translation tests**: `tests/translate/*.pas` → generated Rust compared
   with `insta` snapshots. These are the "which Pascal construct becomes which
   Rust construct" tests the user asked for; snapshots are reviewed by hand
   when the codegen changes.
3. **Execution tests**: `tests/run/*.pas` with optional `*.in` (stdin) and
   required `*.out` (expected stdout). The harness transpiles, builds all of
   them into one scratch workspace (so one `cargo build` covers every test),
   runs, and diffs. Expected outputs are hand-written at first; installing
   Free Pascal (`brew install fpc`, run with `-Mtp`) gives a reference
   implementation to generate them, which is open question #2.

The examples are built in CI (`pascrust build` on every file) and the
non-interactive ones are run. Interactive ones get scripted `*.in` key
streams where feasible.

## 22. FFI

### Pascal → Rust (calling Rust from Pascal)

Use TP's `external` directive: `procedure Beep(freq: integer); external;` in a
unit's interface. Codegen emits `use crate::externs::beep;` and a call with
the normal type mapping (`var` → `&mut`, strings → `&mut PString`). The user
provides `src/externs.rs` in the generated crate (the driver creates a stub
with the expected signatures and never overwrites it). The runtime types are
all public, so Rust can construct `PString`s and `PArray`s.

### Rust → Pascal (embedding a Pascal engine)

`pascrust build --lib unit.pas` emits a library crate whose public API is the
unit's interface (`pub fn`s over runtime types). The host calls
`pascrust_runtime::call(|| bstunit::insert(...)) -> Result<T, PascalError>`,
which catches panics and returns them as an error value. Globals of a unit are
bundled into a `pub struct` the host owns and passes in, which is the §15.4
lifting with the host as `main`. This is the "Pascal crust inside Rust"
configuration from the project's name.

### C

Declare `procedure X(...); external 'C';` (an extension); codegen emits an
`extern "C" { fn x(...); }` block with `#[repr(C)]` on referenced records,
`PString` passed as `*const u8` length-prefixed, and an `unsafe` call wrapped
in a safe shim. Linking uses `cc`/`build.rs` in the generated crate. Tier 3.

### Already-compiled Pascal (`.TPU`/`.EXE`)

Not possible directly: Turbo Pascal output is 16-bit real-mode x86 and cannot
be linked into a 64-bit macOS process. Ideas for later: (a) compile the legacy
unit with Free Pascal in `{$mode tp}` as a C-ABI library and use the C FFI
above; (b) run the original binary under DOSBox and talk to it through a file
or serial-port bridge; (c) transpile it, which is this project. Recorded in
`TODO.md` as ideas only.

## 23. Threads

Turbo Pascal has no threads (DOS is single-tasking), so none of the examples
and no TP program needs them. Decision: skip threads entirely. The runtime is
single-threaded by design: terminal and Dos state are `thread_local!`
(except the flag saying the terminal is in raw mode, an `AtomicBool`, so
the main thread can restore the terminal when the program ends), and
`Ptr<T>` is `Rc`, not `Arc`. The only thread the runtime creates is the
big-stack worker in `run()`, and the Pascal program lives entirely on it.
The one exception is the graphics window, which the main thread services
while the program runs (§19); the screen and key queue it shares with the
program are behind `Mutex`es.

## 24. Worked example

Pascal input (`demo.pas`), chosen to touch a record, a pointer, a `var`
parameter, a nested procedure that captures an outer local, a global, and
formatted output:

```pascal
program Demo;
type
  NodePtr = ^Node;
  Node = record
    value: integer;
    next: NodePtr;
  end;
var
  head: NodePtr;
  i: integer;

procedure Push(var list: NodePtr; v: integer);
var p: NodePtr;
begin
  new(p);
  p^.value := v;
  p^.next := list;
  list := p;
end;

procedure Report(list: NodePtr);
var total: integer;
  procedure Visit(p: NodePtr);
  begin
    while p <> nil do begin
      total := total + p^.value;
      p := p^.next;
    end;
  end;
begin
  total := 0;
  Visit(list);
  writeln('Total: ', total:5);
end;

begin
  head := nil;
  for i := 1 to 3 do Push(head, i * 10);
  Report(head);
end.
```

Expected Rust output (`out/demo/src/main.rs`), after `rustfmt`:

```rust
//! Generated by PascRust from demo.pas. Do not edit by hand.
use pascrust_runtime::prelude::*;

/// Pascal: `Node = record value: integer; next: NodePtr; end`
#[derive(Clone, Default, Debug)]
struct Node {
    value: i16,
    next: NodePtr,
}

/// Pascal: `NodePtr = ^Node`. `Ptr<T>` is the runtime's Pascal pointer:
/// a handle to a heap cell that several pointers may share (`Rc`) and
/// that is borrow-checked at run time (`RefCell`), so a nil or disposed
/// pointer is a clean runtime error, never undefined behaviour.
type NodePtr = Ptr<Node>;

/// Pascal: `procedure Push(var list: NodePtr; v: integer)`
/// `var` parameters become `&mut`; the caller passes `&mut head`.
fn push(list: &mut NodePtr, v: i16) {
    let mut p: NodePtr = Ptr::nil();
    p.new_(); // Pascal `new(p)`: allocate a zeroed Node on the heap
    p.get_mut().value = v; // `p^.value := v`
    p.get_mut().next = list.clone(); // cloning a Ptr copies the handle, not the Node
    *list = p.clone();
}

/// Pascal: `procedure Report(list: NodePtr)`
fn report(list: NodePtr) {
    let mut total: i16 = 0;
    report_visit(&mut total, list);
    pwriteln!("Total: ", w(total, 5)); // `total:5` prints right-aligned in 5 columns
}

/// Pascal: `procedure Visit(p: NodePtr)`, nested inside `Report`.
/// Rust has no nested procedures that can see the outer locals, so the
/// transpiler "lifted" it to the top level and passes `Report`'s `total`
/// explicitly. (This is what every Pascal compiler does internally too.)
fn report_visit(total: &mut i16, mut p: NodePtr) {
    while !p.is_nil() {
        *total = *total + p.get().value;
        // `p := p^.next` needs a temporary: `p.get()` borrows `p` until the
        // end of the statement, so Rust will not let us assign to `p` in it.
        let next = p.get().next.clone();
        p = next;
    }
}

/// The Pascal main program. Globals (`head`, `i`) are locals here and are
/// handed to the procedures that use them.
fn pascal_main() {
    let mut head: NodePtr = Ptr::nil();
    for i in 1..=3_i16 {
        push(&mut head, i * 10);
    }
    report(head.clone());
}

fn main() {
    // run() catches Pascal runtime errors (panics), restores the terminal,
    // prints "Runtime error NNN" like Turbo Pascal, and sets the exit code.
    pascrust_runtime::run(pascal_main);
}
```

Things to notice, because they are the architecture in miniature: the heap is
`Ptr<T>`, not addresses; `var` is `&mut`; the nested procedure was lifted and
its captured variable became a parameter; the global became a local of
`pascal_main`; and the one borrow-checker accommodation (`let next = ...`) is
explained in place.
