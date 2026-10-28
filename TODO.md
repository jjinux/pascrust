# Pascrust TODO

Plan written by Fable (the architect pass). Opus takes over from here. The
core decisions live in `architecture.md`; the language tiers in
`language_spec.md`; prior-art research in `notes/p2c.md`; questions for the
user in `notes/open_questions.md`. Check items off as they land. Every phase
ends with something runnable, and the examples in `examples/` are the
acceptance test for the whole project.

## Original TODOs

- [x] Figure out a name:
  - [x] Pascrust
- [x] Write a TODO.md file.
- [x] Have Fable fill out the TODO.md with the plan and make the core architectural decisions.
- [x] Opus can then read the plan and fill in the details.
- [x] Write a CLAUDE.md.
- [x] Use an MIT license.
- [x] Figure out if I should aim to translate Pascal or Turbo Pascal:
  - [x] Is there an official spec and/or open source implementation for Turbo Pascal? (No formal spec; the TP 7.0 Language Guide is the spec, Free Pascal `-Mtp` is the open-source implementation. See `notes/p2c.md` §4.)
  - [x] Wasn't there some famous Pascal to C compiler? If so, carefully study how it's implemented. Take notes and save them into a file. (p2c; see `notes/p2c.md`.)
  - [x] Write a file called language_spec.md with a plan of what language constructs we want to support. (Decision: Turbo Pascal; tiers in `language_spec.md`.)
- [x] Make the core architectural decisions:
  - [x] Figure out what lexer and parser I should use. (Handwritten recursive descent; `architecture.md` §4.)
  - [x] Maybe we should have a "virtual heap" that represents the Pascal heap in a somewhat safe way. (Passing around a heap object, embedding a Pascal engine in Rust, maybe a memory-mapped file; errors raise to the Rust container.) (Decided: handles (`Ptr<T>` over `Rc<RefCell>`), not a heap object; panics caught by `run()`; `architecture.md` §5, §12, §17.)
  - [x] How should we represent arrays and strings in Pascal in Rust? (`PArray<T, LO, N>` and `PString`; §8, §10.)
  - [x] How should we represent pointers and the heap? (`Ptr<T>`; §12.)
  - [x] How should we represent records? (structs, variant records flattened; §11.)
  - [x] How do we build a FFI to Rust? (§22.)
  - [x] How do we build a FFI to C? (§22, tier 3.)
  - [x] Is it even possible to have a FFI to Pascal that's already been compiled? (Low priority; save ideas for later.) (Not directly; ideas in §22.)
  - [x] Can we skip implementing threads? Do Pascal / Turbo Pascal even have those built in? (Yes, skip; TP has none; §23.)
  - [x] Write all of the core architectural decisions to a file named architecture.md.
- [ ] Start building a battery of tests to verify we're compiling it correctly:
  - [x] Are there existing tests in other open source projects that we can use for inspiration? (Free Pascal `tests/`, p2c `examples/`, Pascal Validation Suite; `notes/p2c.md` §3-4.)
  - [ ] There should be tests that test what Pascal constructs translate into what Rust constructs.
- [ ] How do we build at least a small standard library, including things like printing to stdout?
  - [ ] There should be tests that compile Pascal to Rust and then run the Rust to generate expected output.
- [x] Is there an official standard library for Pascal or Turbo Pascal that has an interface that we can match? (The TP `System` unit plus `Crt`/`Dos`/`Graph`; list in `notes/p2c.md` §5.)
- [x] If possible, copy all the code in ~/Dropbox/programming/turbo_pascal to an examples directory and get them working.
  - [x] Copy the code into `examples/`.
  - [x] Get them working. (All ten programs transpile, build without warnings and run; see Phases 4 and 5.)

## Phase 0: Scaffolding (half a day)

- [x] Review `notes/open_questions.md` with the user; record answers at the top of that file.
- [x] Create the Cargo workspace: `crates/pascrust` (bin + lib) and `crates/pascrust_runtime` (lib with features `crt`, `dos`, `graph`). Edition 2024.
- [x] Write `CLAUDE.md`: project goals, the "intermediate Rust with teaching comments" style rule, where decisions live, how to run tests, the rule that every new Pascal construct needs a translate test and a run test.
- [x] Add `.gitignore` entries for `out/` and `target/`; add `rustfmt.toml`; add a GitHub Actions workflow (fmt, clippy, test).
- [x] Add `tests/README.md` describing the three test layers from `architecture.md` §21.
- [x] `pascrust --help` prints usage; `pascrust emit`, `build`, `run` subcommands stubbed with `clap`.

## Phase 1: Hello world end to end (first thing that runs)

Goal: `pascrust run tests/run/hello.pas` prints `Hello, world!` by going
lexer → parser → AST → sema → codegen → Cargo project → `cargo run`.

- [x] `token.rs`: token enum (keywords, identifiers, integer/real/string/char literals, operators, punctuation) with `Span`.
- [x] `lexer.rs`: handwritten lexer; case-insensitive keywords; `{ }`, `(* *)` and `//` comments; `{$...}` directives kept as a `Directive` token; `1..5` vs `1.5`; `'it''s'`, `#13`, `^M`; hex `$FF`.
  - [x] Unit tests for every token class and the tricky cases.
- [x] `ast.rs`: `Program`, `Block`, `Decl` (const/type/var/proc), `TypeExpr`, `Stmt`, `Expr`, all with spans. Keep it a plain tree of enums and structs; comment what each node corresponds to in Pascal.
- [x] `parser/`: recursive descent for `program`, `const`/`var` sections, `begin ... end`, procedure calls with arguments, string and integer literals, `writeln`. Error type with Pascal file:line:col.
  - [x] `insta` snapshot tests of the AST for hello world and a few malformed inputs (error messages).
- [x] `sema/` skeleton: symbol table with case-insensitive lookup, builtin `writeln`, types `Integer`/`String`; annotates the AST with types.
- [x] `codegen/`: indenting writer; emits `use pascrust_runtime::prelude::*;`, `fn pascal_main()`, `fn main() { pascrust_runtime::run(pascal_main) }`, `pwriteln!(...)`.
- [x] `pascrust_runtime`: `prelude`, `run()` (plain version: call the closure, map panics to exit code 1 for now), `pwrite!`/`pwriteln!` macros over a `PasWrite` trait implemented for `&str`, integers, `f64`, `bool`, `u8`.
- [x] `driver.rs`: writes `out/<name>/Cargo.toml` (path dependency on the runtime, `overflow-checks = false`) and `src/main.rs`; runs `rustfmt` if present; `build`/`run` shell out to `cargo`.
- [x] Test harness v1: `tests/run_tests.rs` finds `tests/run/*.pas`, transpiles each, builds, runs with `*.in` as stdin, diffs against `*.out`. Build all test programs into one scratch workspace so a single `cargo build` covers them.
- [x] Translate test harness v1: `tests/translate_tests.rs` snapshots the generated Rust for `tests/translate/*.pas` with `insta`.
- [x] `hello.pas` passes both harnesses. Commit.

## Phase 2: Core language (tier 1 of `language_spec.md`)

Each bullet: parser + sema + codegen + a translate test + a run test. Order
chosen so each step is testable with the previous ones.

- [x] Integer, real, boolean, char expressions with TP precedence and promotion rules; `div`/`mod`/`/`; `as` casts inserted by sema's type info. Decide and document `i16` wrapping (`architecture.md` §7).
- [x] `var` declarations with `Default` initialization; assignment; `if`/`else` (dangling else); `while`; `repeat`; `for to/downto` over integers.
- [x] `writeln`/`write` formatting: `:w`, `:w:d`, TP default real format, booleans. Run tests compare byte for byte.
- [x] `readln`/`read` from stdin for integers, reals, chars, strings (TP's lenient parsing). `.in` files in run tests.
- [x] Procedures and functions: value params, `var` params as `&mut`, function result variable, `Exit`, recursion, `forward`. Call-site `.clone()` insertion for non-Copy value args.
- [x] Nested procedures: lifting analysis in sema (free variables, transitive through calls, including recursion), codegen of lifted `fn outer_inner(captured..., params...)`. Tests: recursive nested proc, nested proc modifying an outer local, two levels of nesting, nested proc capturing an outer `var` parameter.
- [x] Globals lifted into `pascal_main` and threaded to procedures (same analysis). Test: a procedure that reads a global and is called from another procedure.
- [x] Alias detection in sema: same lvalue root passed twice (`Swap(x, x)`, `F(a, a[1])`, global passed by `var` to a procedure that uses it); report a transpile error with the Pascal position. Tests for each.
  - [x] Alias *unification*: when every call passes the same outer variable for a `var` parameter that the callee also captures (`life.pas`'s `Pause`), use the parameter for both instead of rejecting the program.
  - [ ] Copy-in/copy-out for `Swap(a[i], a[j])` on arrays of `Copy` elements (currently rejected because both roots are `a`).
- [x] `PString`: runtime type (`[u8; 255]` + `len` + `cap`), `Index<i32>`, concatenation, comparison, `assign` with truncation, `pstr!` literal macro; string vs char coercions in sema. `Length`, `Copy`, `Pos`, `Delete`, `Insert`, `Concat`, `UpCase`, `Str`, `Val`.
- [x] Enumerated types and `Ordinal` trait (generated impl per enum; runtime impls for integers, `u8`, `bool`); `Ord`, `Chr`, `Succ`, `Pred`, `Inc`, `Dec`, `Low`, `High`; `for` over enums and chars.
  - [ ] `Low`/`High`, and `Inc`/`Dec` on enums.
- [x] Subranges as host types.
  - [ ] `--range-checks` flag.
- [x] `case` → `match` with lists, ranges, `else`, and no-match-is-no-op.
- [x] `PArray<T, LO, N>`: runtime type, `Index`/`IndexMut<i32>`, `Default` via `array::from_fn`, `Copy` when `T: Copy`; multi-dimensional sugar; char/enum indexes. Range-check panic message in TP style.
- [x] Records: struct generation, `Clone`/`Default`/`Copy` decisions, field access, record assignment, records in arrays, arrays in records, anonymous record types in `var`.
- [x] `const` section with constant folding; typed constants (array and scalar) as initialized variables.
- [x] Math and misc builtins: `Abs Sqr Sqrt Sin Cos ArcTan Ln Exp Trunc Round Int Frac Pi Odd Random Randomize Hi Lo Swap MaxInt`.
- [x] `Halt`, `Halt(n)`, exit codes; runtime `run()` with `catch_unwind`, TP-style "Runtime error NNN" messages and codes (200, 201, 204, ...), big-stack worker thread.
- [x] Naming: case-insensitive canonicalization, `snake_case`/`CamelCase` conversion, Rust keyword escaping, user declarations shadowing builtins (`procedure Exit`, `type ShortString`). Test with the examples' names.
- [x] `///` comments with the original Pascal declaration on every generated item; teaching comments at first use of `Ptr`, const generics, lifting, `catch_unwind`.
- [x] Milestone: `magic_square.pas` (minus `Crt`) and the tier-1 subset of `paper.pas` run.

## Phase 3: Files, units, objects, pointers (tier 2 language)

- [x] `Text` files: `PText` runtime type; `Assign`/`Reset`/`Rewrite`/`Append`/`Close`/`Eof`/`Eoln`/`Erase`/`Rename`/`Flush`; `read`/`write` variants taking a file. `SeekEof`/`SeekEoln`; `{$I-}`/`IOResult`. Run tests that write then read a temp file.
- [x] Typed files: `PFile<T>` with a generated `PascalRecord` (serialize/deserialize) impl per record type; `Seek`/`FilePos`/`FileSize`/`Truncate`. Test with a `file of record` round trip and `Seek` to the middle.
- [x] Units: driver locates `uses` targets (same directory, then `-I` paths), parses interface/implementation, emits one module per unit with `pub` visibility, `init()` for initialization sections called in `uses` order; name resolution across units with TP's later-unit-wins rule. Test: a program using two units with a name clash.
- [x] Objects: `struct` + `impl`, `&self`/`&mut self` selection, implicit `Self` field access, nested procedures inside methods (`this: &mut T` lifting), inheritance by composition (`base` field, path rewriting, upcasting to `&mut child.base`), `private` ignored. Tests from `bstunit.pas` and `clock/menu.pas` patterns.
- [x] Pointers: `Ptr<T>` runtime type (`nil`, `new_`, `dispose` with use-after-dispose detection, `get`/`get_mut`, `ptr_eq`), temporaries hoisted for `p := p^.next` and for call arguments containing derefs, `var` args of heap fields as `&mut p.get_mut().field`. Tests: linked list push/pop, BST insert/delete (the `bstunit.pas` algorithm verbatim), `Root^ := TempPtr^`.
- [x] `RefCell` double-borrow panics mapped to a clear "pointer aliasing" runtime error message with the Pascal procedure name.
- [x] `with` statements by path rewriting (sema rewrites the body; `with p^` copies the pointer when the body could change it; changing an index that chooses the record is an error).
- [x] `{$I-}` and a real `IOResult`: I/O calls in a `{$I-}` region are wrapped in `io_unchecked`, which keeps the error code for `IOResult`.
- [x] Hoist pointer dereferences out of call arguments too, so `f(p^.x)` never holds a `RefCell` guard across the call.
- [ ] Methods that don't modify `Self` could take `&self` instead of `&mut self`.
- [ ] `Break`/`Continue`; `FillChar`/`Move` on arrays of scalars; `ParamCount`/`ParamStr`; `SizeOf` for sized types; `{$N+}` float types.
- [x] Milestone: `examples/database/` (bstunit, founit, filesys) transpiles and the non-interactive paths run (with `Crt` stubbed to plain mode).

## Phase 4: Crt, then the text-mode examples

- [x] `crt` feature: `crossterm`-backed terminal with TP state (`TextAttr`, `WindMin`/`WindMax`), `ClrScr`, `ClrEol`, `GotoXY`, `WhereX`/`WhereY`, `TextColor`/`TextBackground`/`HighVideo`/`LowVideo`/`NormVideo`, `Window`, `InsLine`/`DelLine`, `TextMode`, `Delay`, `Sound`/`NoSound`, `KeyPressed`, `ReadKey` (with `#0` + scan codes for arrows/function keys), colour constants, `Blink`.
- [x] Route all `write` output through the runtime (raw-mode `\r\n`, CP437).
  - [ ] `Window` clipping and scrolling of output (currently `Window` only affects `ClrScr`/`GotoXY`/`WhereX`).
- [x] CP437 → Unicode output mapping; reverse mapping on input.
- [x] Plain mode when stdout is not a TTY or `PASCRUST_TERM=plain`: cursor ops are no-ops, `ReadKey` reads stdin bytes, `Delay` honours `PASCRUST_FAST`.
- [x] Scripted-input run tests: in plain mode `ReadKey` reads the test's `.in` file; compare stdout (`tests/run/crt_plain.pas`).
- [x] Get working, one at a time, fixing transpiler bugs as found: `gmscode.pas`, `paper.pas`, `magic_square.pas`, `life.pas`, `blkjack.pas`, `menu.pas` (needs `Dos.Exec`, which may fail gracefully), `airport/` (four units). All transpile and build warning-free; `magic_square`, `paper`, `airport4` and `database/filesys` verified with scripted sessions.
  - [ ] Turn those scripted sessions into run tests (`examples/` acceptance tests in CI).
  - [x] `examples/database/founit.pas` empties the master file on delete: `Fcopy` stops at `eof` because nothing seeks back to the start. Confirmed as a bug in the original program and fixed there (plus `BST.ShiftLocations`, since later records move up a position).
  - [x] `bstunit.pas`: deleting a node with two children copied the successor's links (`Root^ := TempPtr^`). Fixed in the example.
- [ ] CI builds every example; runs the non-interactive ones.

## Requests from the user, 2026-10-27

- [x] Fix the two bugs found in the user's examples (`bstunit`, `founit`). Fixing them exposed a transpiler bug: call arguments that read through a pointer kept the record borrowed during the call; now they're copied into a `let` first (`tests/run/ptr_args.pas`).
- [x] README: usage instructions.
- [x] README: an "Inspiration" section.
- [x] BGI graphics (Phase 5 below). `clock/` and `tictac/` run. The clock's hands go round anticlockwise on purpose: it's a playful backwards clock.
- [x] Consistent identifier casing in the examples.
- [x] Consistent spacing in the examples: `pascrust fmt` (`crates/pascrust/src/fmt/`), a re-indenter that reuses the lexer; the examples are kept formatted by `tests/fmt_tests.rs`. Prototyped in Python, then ported.
- [x] DOS and UNIX line endings both work: source, standard input and text files (`tests/run/crlf.pas`, stored with CRLF; see `.gitattributes`), and `fmt`.
- [x] Code review by Fable (two reviewers: transpiler; runtime and `fmt`). Fixed: `Continue` in `repeat` skipped the `until` test; hoisted temporaries could collide or shadow variables; scoped `for` loops with literal bounds became `i32`; `-b` on `Byte`/`Word`; big subranges; `Ord`/`Abs` in constants; `Write` of an enum; `Boolean(n)`; `^Name` resolved too early; `shr` was arithmetic; `Read` into `string[n]`; directives silently ignored (now warnings, includes an error); pointer comparisons cloned; `get_mut` on every `^` in a path; `if` temporaries in a block; Crt left the terminal raw after a normal end or runtime error; `Val` crashed on non-ASCII; reals rounded ties to even and `Inf` crashed; Graph-only programs built with warnings; `fmt` and numeric labels; `Eof` at an unterminated last line; doc drift. Still open from the review:
  - [x] `var p: ^T;` with a one-letter type name lexed as the control character `^T`. Now, as in Free Pascal, `^` is the pointer symbol wherever a type can appear (`type` and `var` sections, a typed constant's type); the lexer tracks which section it is in. Like FPC, `set of ^A..^Z` in a `type` section is then a pointer, not characters.
  - [x] Two disjoint fields of one record as two `var` arguments (`Init(p.x, p.y)`) were rejected. Sema now compares field paths; two fields of one heap record (`q^.x, q^.y`) borrow it once with `Ptr::with_mut`. Still rejected: two elements of one array (`a[1], a[2]`).
  - [ ] A local that clashes with a captured global renames the global everywhere (`global_count`); scope the rename to the procedure with the clash.
  - [ ] Hoisting a deref argument evaluates it before earlier arguments with side effects (`Show(Bump, head^.value)`).
  - [x] `x shl n` with `n` at least the type's width didn't compile (constant) or panicked (variable); TP gives 0. Now `checked_shl(n).unwrap_or(0)` unless the count is a literal below the width. A shift also has the type of its left operand now, as in TP.
  - [ ] `p^.x := p^.x + 1` hoists a temporary instead of becoming `p.get_mut().x += 1`.
  - [x] `Random(n)`: checked against a disassembly of TP's System unit (github.com/daelsepara/turbo-pascal-assembly): it uses the full 32-bit seed, `(seed * n) shr 32`, as we do. `n` is now truncated to a `Word`, as TP's parameter is. Not done: `RandSeed` as an assignable variable.
  - [x] `{$I-}` and a real `IOResult`.
  - [x] `SeekEof`/`SeekEoln`.
  - [ ] `LastMode` (was listed as done; it isn't).
  - [ ] `Val` corner cases: `Val('40000', i, code)` for an `Integer` is an error for us, wraps (code 0) in TP/FPC.
  - [ ] Graph: should `SetBkColor` use the current `SetPalette` entry? Do XOR-mode rectangles lose their corners in the real BGI? `FExpand` ignores `ChDir`.
  - [ ] `i16::MIN div -1` panics (runtime error 255); TP wraps.
  - Not changed: `Round(2.5)` is 3. The reviewer suggested ties-to-even to match Free Pascal, but Turbo Pascal's documentation says halves round away from zero. `Eof` on a text file being written stays runtime error 104, as in Free Pascal (the reviewer thought TP returns True).

## Phase 5: Dos and Graph, then the graphics examples

- [x] `dos` feature (the parts the examples use; `FSplit`, `GetDir`, `DosVersion`, `FindFirst`/`FindNext`, `DiskFree` still to do): `GetDate`/`GetTime` (`chrono`), `FExpand`/`FSearch`/`FSplit` with the drive-letter mapping and `\`→`/` translation, `Exec`/`DosExitCode` via `std::process::Command`, `ChDir`/`MkDir`/`RmDir`/`GetDir`, `GetEnv`, `DosVersion`, `FindFirst`/`FindNext`, `DiskFree`/`DiskSize` best effort; `PathStr` and friends as `PString` aliases.
- [x] `graph` feature, headless core first: 640x480 framebuffer of colour numbers, EGA palette (`SetBkColor`, `SetPalette`), `PutPixel`/`GetPixel`, Bresenham `Line`/`LineTo`/`LineRel`, `Rectangle`, `Bar`/`Bar3D` with 8x8 fill patterns (`SetFillStyle`/`SetFillPattern`/`GetFillPattern`), `Circle`/`Arc`/`Ellipse`/`FillEllipse`/`PieSlice`/`Sector`, `FloodFill`, line styles and `ThickWidth`, viewports (`SetViewPort`/`ClearViewPort`/`ClearDevice`), `SetWriteMode`.
  - [ ] `GetImage`/`PutImage`/`ImageSize`, `DrawPoly`/`FillPoly`, `GetArcCoords`, `GetLineSettings` and the other `Get*Settings` records.
- [x] Bitmap text: the public-domain 8x8 VGA font (ASCII); `OutText`/`OutTextXY`/`SetTextJustify`/`SetTextStyle` (default font, integer scale; stroked fonts approximated with it)/`TextWidth`/`TextHeight`.
  - [ ] CP437 characters 128-255 in the font (box drawing, accented letters).
- [x] `InitGraph`/`Detect`/`DetectGraph`/`CloseGraph`/`GraphResult`/`GraphErrorMsg`/`SetGraphMode`/`GetGraphMode`/`RestoreCrtMode`/`GetMaxX`/`GetMaxY`/`GetMaxColor`/`GetAspectRatio`.
- [x] Tests: `PASCRUST_GRAPH=headless` draws without a window; `tests/run/graph_basics.pas` checks drawings with `GetPixel` (instead of golden images; see architecture.md §19). `PASCRUST_GRAPH_SAVE=file.bmp` saves the screen for a look by eye.
- [x] `minifb` window backend: present the framebuffer, forward key events to `ReadKey`/`KeyPressed` while graphics mode is active, window serviced by the main thread (macOS needs that) while the program runs on its worker thread.
- [x] Get working: `clock/` (menu unit, clock unit, tester), `tictac/` (five units, `Ollie`, `CompUnit` AI). Checked headless with scripted keys and saved screenshots; the window opens on macOS.
  - [x] Transpiler fixes they needed: method call arguments that use the receiver (`obj.Prompt(obj.WhoseTurn)`) are computed first (`tests/run/method_args.pas`); two anonymous record types for same-named variables are `EndPoint` and `EndPoint2`, not `EndPoint_2` (`tests/run/anon_records.pas`).
  - [x] DOS drives: an unmapped drive is the directory the program started in, not the current one (`tictac` does `ChDir('a:\TicTac')` before every game), and names match files on disk case-insensitively.
  - [ ] Play a whole game against the computer in the window, by hand.
- [x] Milestone: every program in `examples/` transpiles, builds without warnings, and runs. Mark the original "Get them working" TODO done.

## Phase 6: Polish and readability

- [ ] Read through the generated Rust for `life.pas` and `bstunit.pas` as if reviewing a colleague's code; fix anything an intermediate Rust programmer would not write (unneeded clones, casts, parentheses, `mut`).
- [ ] Commit showcase generated output for `hello.pas` and `life.pas` (per open question 7) with a refresh script.
- [ ] Carry Pascal comments into the Rust output at statement boundaries.
- [ ] Make transpiler diagnostics show the source line with a caret; add `--explain` text for the alias-detection error.
- [ ] Write `docs/translating.md`: a user guide with the Pascal→Rust mapping table and the known limitations, generated partly from the translate-test snapshots.
- [ ] Measure and trim compile time of generated crates (feature flags, no unused dependencies).

## Phase 7: Tier 3 and FFI (as interest dictates)

- [ ] Sets (`PSet<T>`, literals, `in`, operators).
- [ ] Variant records (flattened).
- [ ] Virtual methods, constructors, destructors. Plan in `notes/virtual_methods.md`: a trait per hierarchy for dispatch (through `Self` and `var` parameters) and an enum per hierarchy that every pointer into it points to, joined by implementing the trait for the enum. About 4 days.
- [ ] Procedural types and procedure parameters.
- [ ] Structured `goto` (forward jumps out of loops → labeled `break`).
- [ ] TP-exact binary file layout including 6-byte `Real`, so `examples/database/mark.dat` can be read.
- [ ] `GetMem`/`FreeMem`/untyped `Pointer` via a place-descriptor representation; the per-procedure "place" option for `var` parameters that hit `RefCell` aliasing.
- [ ] Bundled-globals option (`struct Globals` per unit) for programs with very many globals.
- [ ] FFI to Rust: `external` procedures mapped to `src/externs.rs` stubs; a worked example calling a Rust function from Pascal.
- [ ] Embedding: `pascrust build --lib unit.pas` and `pascrust_runtime::call(...) -> Result`, with a host example that calls `BSTUnit` from Rust.
- [ ] FFI to C: `external 'C'` → `extern "C"` block with `#[repr(C)]` records, `build.rs` with `cc`.
- [ ] Ideas only, for already-compiled Pascal: Free Pascal `{$mode tp}` C-ABI library, or DOSBox bridge. Not scheduled.
- [ ] `--integer-width 32` flag if the user wants it.

## Testing track (runs alongside every phase)

- [ ] One translate snapshot and one run test per construct in `language_spec.md` tier 1 and 2; keep a checklist table in `tests/README.md`.
- [x] If FPC is installed (open question 2): script `tests/gen_expected.sh` that compiles each `tests/run/*.pas` with `fpc -Mtp` and records its stdout as `*.out`; commit the outputs.
- [ ] Import a curated subset of Free Pascal `tests/test/t*.pp` programs that are TP-mode and self-checking (exit code 0), with attribution.
- [ ] Import p2c's `examples/*.p` where they parse as TP.
- [ ] Negative tests: malformed programs produce the expected diagnostic text (snapshot).
- [ ] Alias-detection tests (should fail to transpile) and `RefCell` aliasing tests (should fail at run time with the clear message).
- [ ] `cargo clippy -- -D warnings` on the transpiler and the runtime; `#![deny(warnings)]` is not applied to generated code, but generated code must build warning-free for every example.
