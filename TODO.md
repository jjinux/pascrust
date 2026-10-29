# PascRust TODO

Plan written by Fable (the architect pass). Opus takes over from here. The
core decisions live in `architecture.md`; the language tiers in
`language_spec.md`; prior-art research in `notes/p2c.md`; questions for the
user in `notes/open_questions.md`. Check items off as they land. Every phase
ends with something runnable, and the examples in `examples/` are the
acceptance test for the whole project.

## Original TODOs

- [x] Figure out a name:
  - [x] PascRust
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

## Requests from the user, 2026-10-28

- [ ] Evaluation order where a function has side effects (the user: follow Free Pascal). Done: `Write(F, G)` writes F before running G; `n := n + Bump` reads `n` first. Not yet, and worth asking whether to chase: on aarch64 Free Pascal calls the function first for reals, strings and sets (`t := t + Ext` where `Ext` changes `t` gives `ayz`, PascRust `az`), and works out an assignment's target index before the right side (`a[Idx] := F` runs Idx first; Rust runs F first). These are Free Pascal code-generator details, and may differ on x86.
- [x] Rename the project to "PascRust" wherever a friendly name fits: the README, the other docs, `Cargo.toml` descriptions, the GitHub repo description, CLI help text and messages, comments and generated-code headers. The directory, the crates, the binary (`pascrust`), the repo name and other identifiers stay `pascrust`.

## Phase 5: Dos and Graph, then the graphics examples

- [x] `dos` feature (the parts the examples use; the rest is in Phase 7): `GetDate`/`GetTime` (`chrono`), `FExpand`/`FSearch`/`FSplit` with the drive-letter mapping and `\`→`/` translation, `Exec`/`DosExitCode` via `std::process::Command`, `ChDir`/`MkDir`/`RmDir`/`GetDir`, `GetEnv`, `DosVersion`, `FindFirst`/`FindNext`, `DiskFree`/`DiskSize` best effort; `PathStr` and friends as `PString` aliases.
- [x] `graph` feature, headless core first: 640x480 framebuffer of colour numbers, EGA palette (`SetBkColor`, `SetPalette`), `PutPixel`/`GetPixel`, Bresenham `Line`/`LineTo`/`LineRel`, `Rectangle`, `Bar`/`Bar3D` with 8x8 fill patterns (`SetFillStyle`/`SetFillPattern`/`GetFillPattern`), `Circle`/`Arc`/`Ellipse`/`FillEllipse`/`PieSlice`/`Sector`, `FloodFill`, line styles and `ThickWidth`, viewports (`SetViewPort`/`ClearViewPort`/`ClearDevice`), `SetWriteMode`.
  - [ ] The rest of Graph: see Phase 7.
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

- [x] Have Claude Fable review every commit since its last code review (`git log 38138d3..HEAD`): three reviewers (language features, the rest of the transpiler, the runtime), every finding reproduced. Fixed:
  - Wrong output: virtual dispatch for `p^` passed to a `var` parameter of an ancestor type; `Random(n)` returned negative values for ranges over 32767 (it's a `Word`); `High`/`Low` of a subrange-typed variable; `FileMode := $40` opened files read-write; `GetDir` above drive C:'s root; printer output lost on Ctrl-C or a signal.
  - Didn't compile or panicked: `Output`/`Input`/`Lst` passed as `var Text`; a static method calling virtual ones, declared below the root, through a pointer; two `var` arguments in one heap record through an index (`arr[i]^.x, arr[i]^.y`; runtime error 204); a pointer and a field behind it as two `var` arguments; sets in `file of` records; `FillChar(b, SizeOf(b), True)`; `Swap($ABCD)`.
  - Missing checks: `New`/`Dispose` with a method that isn't a constructor/destructor; a `with` field as a `for` variable; descendant objects by value (now allowed, copying the ancestor part); `pascrust emit game.pas` without a directory; `SetTextBuf` made its file `mut` (a warning).
  - Tests: run tests and a new examples test now build generated code with `-D warnings`, so warnings fail the suite.
  - Decided with the user: a value outside a set's range is never in it, `300 in s` is False for a constant and a variable alike (FPC truncates the variable).
  - Open: `SizeOf` of a subrange variable (2 here, 1 in TP), of a small set (32 here), and of an object with virtual methods (TP adds 2); a `{$I-}` error still waiting isn't raised by later `{$I+}` I/O (documented in architecture.md); `x shl 32` is 0 where TP masks the count to 5 bits.
- [ ] Open from the Fable reviews of the fix commits (2026-10-28): a function or method call inside an assignment's target index (`arr[Pick] := ...`, where `Pick` reads `arr`, or `items[Count]` inside a method) doesn't borrow-check, since the call is passed the array or `self` while the place is borrowed; hoist the index into a temporary. TP also evaluates `a[F] := a[F] + 1`'s index twice, where `a[f()] += 1` does it once. And: in `s := s + Bump` where `Bump` changes `s`, FPC calls `Bump` first and PascRust reads `s` first (TP's order unverified); `i in [-3..-1]` (set elements outside 0..255) depends on whether the ends are literals or named constants; `b in [True]` is `matches!(b, true)`, which clippy flags. Cosmetic: an enum value named like a runtime type gets a needless `_` (`Kind::Ptr_`); a long record's `/// Pascal:` line quotes the Rust name (`FileRecord_ = record ... end`). Two Pascal names that become the same Rust name in different scopes (a variable `enum_range` and a function `EnumRange`) clash.
- [ ] Announce PascRust to the Pascal community: find which Pascal compiler projects are still alive (Free Pascal, Lazarus, and others), find their mailing lists or forums and what each allows for announcements, then draft a short announcement for each (what PascRust is, that it's MIT, a link to the repository). The user reviews and sends them; Claude doesn't post anything itself.
- [x] Add an "Acknowledgements" section to the README thanking the projects we learned from and compared against. Our code is our own, but being able to run their tools and compare output helped a great deal: Free Pascal (`fpc -Mtp` produces most of our expected test output), daelsepara's turbo-pascal-assembly (the System unit disassembly behind `Random`), J R Stockton's Pascal pages, Borland's Turbo Pascal manuals, p2c (design ideas), and the Rust crates we build on (`clap`, `crossterm`, `minifb`, `insta`, ...). Also Claude Fable, for the code and design reviews.
- [x] `tpc`: a command-line compiler with Turbo Pascal's `TPC.EXE` options (`-$`, `-E`, `-U`, `-Q`; the rest accepted and ignored). Not an `fpc` look-alike, which would hide the real one.
- [ ] Under `{$I-}`, run each `Write`/`Writeln` argument as its own unchecked call, as TP does: in `WriteLn('x', IOResult)` with an error waiting, TP skips the `'x'` but prints the result (FPC prints `2`; we print nothing).
- [ ] Keep the generated Rust reasonably readable by a human. It's fine for it to call helper functions in the runtime (`pascrust_runtime`) when a Pascal construct needs support code; what matters is that a reader can follow the translated program.
  - [ ] `and`/`or`/`xor` of two Byte values compared with a constant that fits: `s.Attr and Directory = Directory` comes out as `i16::from(s.attr) & i16::from(DIRECTORY) == i16::from(DIRECTORY)`; `s.attr & DIRECTORY == DIRECTORY` computes the same (found by the Fable review of FindFirst).
- [ ] Read through the generated Rust for `life.pas` and `bstunit.pas` as if reviewing a colleague's code; fix anything an intermediate Rust programmer would not write (unneeded clones, casts, parentheses, `mut`).
- [ ] Commit showcase generated output for `hello.pas` and `life.pas` (per open question 7) with a refresh script.
- [ ] Make transpiler diagnostics show the source line with a caret; add `--explain` text for the alias-detection error.
- [ ] Write `docs/translating.md`: a user guide with the Pascal→Rust mapping table and the known limitations, generated partly from the translate-test snapshots.
- [ ] Measure and trim compile time of generated crates (feature flags, no unused dependencies).

## Phase 7: The rest of Turbo Pascal 7's standard units

Borland's *Turbo Pascal 7.0 Programmer's Reference* lists every routine,
variable and constant of the standard units, so "done" is well defined. The
order below puts first what old programs use most. Where the real thing
makes no sense on a modern machine, we **fake it**: accept the call and do
something harmless and plausible, so the program runs, and list the fake in
`language_spec.md`. Each item gets translate and run tests, checked against
`fpc -Mtp` where FPC matches TP.

Next up (moved here from Phase 6 at the user's request)
- [x] Carry Pascal comments into the Rust output at statement boundaries: anchored to statements, declarations, record fields, procedures and block ends (`codegen/comments.rs`, architecture.md §20).

System, the small routines
- [x] `SizeOf` with TP's sizes (`Integer` 2, `LongInt` 4, `Real` 6, `Char` and `Boolean` 1, `string[n]` n+1, records unpadded, arrays), so `FillChar(a, SizeOf(a), 0)` and record-size arithmetic work.
- [x] `High`/`Low` (of arrays, strings, ordinal types and variables), `Swap`, `Assigned`, `RunError(n)`. `SizeOf`, `High` and `Low` are folded into constants, as in TP, so they work in `const` declarations and array bounds. (`Double`, `Single` and `Extended` are all `Real` here, so `SizeOf` gives them 6.)
- [x] `FillChar` and `Move` for the common cases: filling a whole array, record or string with one byte value (zeroing is by far the most common), and copying between variables of the same type. Other uses (partial byte ranges, mismatched types) are a clear error.
- [x] `GetDir(drive, s)`: the current directory as a DOS path, through the drive mapping.
- [x] System variables: `ExitCode` (read/write; `Halt` without an argument uses it), `RandSeed` (read/write, so `RandSeed := 42` gives a repeatable sequence), `FileMode` (0 opens typed files read-only), and `Input`/`Output` as `Text` variables (`Writeln(Output, ...)`). Done as runtime variables (`SymKind::RuntimeVar`: a getter and a setter, `set_rand_seed(42)`), which Crt's `TextAttr` and `LastMode` can reuse; `Input`/`Output` are always the console (reopening them is an error).
- [x] Fake: `MemAvail`/`MaxAvail` return a typical DOS figure (about 500 KB), so "is there enough memory?" checks pass; `Mark`/`Release` do nothing (records are freed when nothing points to them); `SetTextBuf` does nothing (our files are buffered anyway); `Test8087` is 3 (an 80387 is present).

Crt, the variables
- [x] `TextAttr` as an assignable variable (now a read-only function), `LastMode` (the mode from `TextMode`), `AssignCrt(f)` (a `Text` file that writes through Crt, honouring the window and colours).
- [x] Fake: `DirectVideo`, `CheckSnow` and `CheckEOF` are accepted and ignored; `CheckBreak` decides whether Ctrl-Break ends the program (as now, by default).

Printer and Overlay
- [x] Fake `Printer`: `Lst` is a `Text` file appended to `printer.txt` (or the file named by `PASCRUST_PRINTER`), so printed reports can be read. `PASCRUST_PRINTER='|lp'` pipes it to a command instead, for a real printer.
- [x] Fake `Overlay`: `OvrInit`, `OvrInitEMS`, `OvrSetBuf`, `OvrGetBuf`, `OvrClearBuf` do nothing and `OvrResult` is 0 (`ovrOk`); `{$O+}` and `{$F+}` are ignored silently. Overlays only existed to fit in 640 KB.

Sets (a language feature, but `Include`/`Exclude` and `in` are everywhere in old code)
- [x] `set of T` as a runtime `PSet` (a 256-bit set, as in TP), set literals with ranges, `in` (a `matches!` for a list of constants), `+ - *` as `| & -`, `= <> <= >=`, `Include`/`Exclude`, set constants (`PSet::of`/`from_ranges`, `const fn`s), sets in typed constants, records and parameters.

Dos, the rest
- [x] `FindFirst`/`FindNext`/`SearchRec` with DOS wildcards (`*.PAS`, `?`), the attribute filter (`Directory`, `AnyFile`, ...), names shown as on disk.
- [x] `FSplit`, `EnvCount`/`EnvStr`, `DosVersion` (fake: DOS 6.22, `$1606`). `DosError` is now a variable.
- [x] `GetFTime`/`SetFTime` with `PackTime`/`UnpackTime` and the `DateTime` record; `GetFAttr`/`SetFAttr` (read-only from the file's permissions, directory from its type; hidden/system/archive faked from the name and kept in memory).
- [x] `DiskFree`/`DiskSize`, faked (as the user suggested): real disks overflow a `LongInt`, so every drive is a 1 GB disk with 512 MB free; past Z: is -1.
- [x] Fake: `SetDate`/`SetTime` don't change the machine's clock; `GetCBreak`/`SetCBreak` and `GetVerify`/`SetVerify` remember a flag; `SwapVectors` does nothing.
- [x] Never: `Intr`, `MsDos`, `Keep`, `GetIntVec`/`SetIntVec` are a clear "needs real DOS" error (`UnitRoutine::refused`).

Graph, the rest
- [x] `DrawPoly`/`FillPoly` (arrays of `PointType`), `GetArcCoords`, and the `Get*Settings` records (`GetLineSettings`, `GetFillSettings`, `GetTextSettings`, `GetViewSettings`).
- [ ] `GetImage`/`PutImage`/`ImageSize` with the `PutImage` modes (`NormalPut`, `XORPut`, ...), storing images in a byte buffer the program allocates. Programs allocate it with `GetMem(p, ImageSize(...))` and pass `p^` (an untyped `var`), so this waits for a design for `GetMem`/`FreeMem` and untyped `Pointer`, shared with untyped files below.
- [x] Palettes: `GetPalette`, `GetDefaultPalette`, `SetAllPalette`, `SetRGBPalette`, `GetPaletteSize`; `GraphDefaults`; `SetAspectRatio`; `SetUserCharSize`; `GetDriverName`/`GetModeName`/`GetMaxMode`/`GetModeRange`.
- [x] `SetActivePage`/`SetVisualPage` (two framebuffers, for flicker-free animation).
- [x] Fake: `InstallUserFont` and `SetGraphBufSize` succeed and change nothing (there are no `.CHR` files to load). `RegisterBGIdriver`, `RegisterBGIfont` and `InstallUserDriver` are refused instead, with the reason: they take `@DriverProc` (code linked in from an `.OBJ` file) or a driver file, and the call can simply go.

Last: raw bytes and C strings
- [ ] Untyped files (`file` without `of`) with `BlockRead`/`BlockWrite`, and untyped `var` parameters, as byte views of records and arrays (with `SizeOf`).
- [ ] `PChar` and the `Strings` unit (`StrLen`, `StrCopy`, `StrCat`, `StrComp`, `StrPas`, `StrPCopy`, ...), as a handle to a byte buffer plus an offset, since `PChar` arithmetic is pointer arithmetic. Then `WinDos` (the `Dos` routines with `PChar` arguments) is mostly aliases.
- [ ] Skip: `Graph3` and `Turbo3` (Turbo Pascal 3.0 compatibility).

## Phase 8: Tier 3 and FFI (as interest dictates)

- [ ] Variant records (flattened).
- [x] Virtual methods, constructors, destructors: a trait per object in the hierarchy for dispatch, an enum per hierarchy for pointers (`notes/virtual_methods.md`, architecture.md §16), and `TypeOf`.
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
- [ ] Write our own self-checking run tests (exit code 0 when every check passes) for the System, Crt, Dos and Graph routines, covering the kinds of cases Free Pascal's test suite covers. Don't copy FPC's or p2c's tests: both are GPL and this project is MIT. Running `fpc -Mtp` as an external program to produce expected output (`tests/gen_expected.sh`) is fine.
- [ ] Negative tests: malformed programs produce the expected diagnostic text (snapshot).
- [ ] Alias-detection tests (should fail to transpile) and `RefCell` aliasing tests (should fail at run time with the clear message).
- [ ] `cargo clippy -- -D warnings` on the transpiler and the runtime; `#![deny(warnings)]` is not applied to generated code, but generated code must build warning-free for every example.
