# PascRust TODO

Plan written by Fable (the architect pass). Opus takes over from here. The
core decisions live in `docs/architecture.md`; the language tiers in
`docs/language_spec.md`; prior-art research in `notes/p2c.md`; questions for the
user in `notes/open_questions.md`. Check items off as they land. Every phase
ends with something runnable, and the examples in `examples/` are the
acceptance test for the whole project.

An item marked "PUNT:" won't be done in the short term (the user, 2026-11-06):
it's kept for the record and checked off, so the open boxes count only what's
planned. The goal is to get to done.

## For the user (things Claude can't do)

Claude can't watch a window, press keys in a real terminal, or run the real
Turbo Pascal. These need you; tell Claude what you find, and it fixes it.

- [x] Play each interactive example by hand with `cargo run -p pascrust -- run <file>` and note anything that looks or behaves differently from how you remember it:
  - [x] Text mode (Crt). Done: `database/filesys.pas` (the user, 2026-11-06: "working"), `airport/airport4.pas` (the user, 2026-11-06: "works fine"), `blkjack.pas` (the user, 2026-11-05: "It's great!"), `life.pas` ("Life works"), `gmscode.pas` ("works"), `paper.pas` ("works :)"), `magic_square.pas` (works), `menu.pas` ("menu works", with its games built into `A:\SHANNON` as the README's Examples section shows).
  - [x] Graphics (a window opens). Done: `clock/` (the user, 2026-11-06: "works"), a whole game of `tictac/` against the computer ("tictac works").
  - [x] PascRust's own examples in `examples/pascrust/`. Done: `rust_calls_pascal/` (the user, 2026-11-06: works, but what it does wasn't clear; the README says now), `calls_rust.pas` (the user, 2026-11-06: "works"), `turtles.pas` (the user, 2026-11-06: "works. Nice job!").
- [ ] If you still have Turbo Pascal 7 (in DOSBox, say), settle what Free Pascal can't tell us, since it works in 32/64 bits:
  - [ ] `w + 1` with `w: Word = 65535`: 0 (PascRust) or 65536?
  - [ ] With `{$Q+}`, is `Inc(b)` at 255 for a Byte (or `w + 1` at 65535 for a Word) error 215, as PascRust says, or 0? (TP's `INTO` tests the signed overflow flag, so it may only catch `Inc(b)` at 127, says Fable.)
  - [ ] `s := s + Bump` where the function `Bump` changes `s`: which runs first?
  - [ ] `type Small = 0..10; const X = Small(300);`: an error, or 44 (PascRust)?
  - [ ] Graph: does `SetBkColor` use the current `SetPalette` entry? Do XOR-mode rectangles lose their corners?
- [ ] Read README.md and docs/ as a newcomer would, and say what's confusing or missing.
- [ ] Announce PascRust to the Pascal community (the user, 2026-11-06: theirs to do): find which Pascal compiler projects are still alive (Free Pascal, Lazarus, and others), their mailing lists or forums and what each allows for announcements, and send a short announcement to each (what PascRust is, that it's MIT, a link to the repository). Claude can help draft them if asked.

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
  - [x] Write a file called language_spec.md (now in docs/) with a plan of what language constructs we want to support. (Decision: Turbo Pascal; tiers in `docs/language_spec.md`.)
- [x] Make the core architectural decisions:
  - [x] Figure out what lexer and parser I should use. (Handwritten recursive descent; `docs/architecture.md` §4.)
  - [x] Maybe we should have a "virtual heap" that represents the Pascal heap in a somewhat safe way. (Passing around a heap object, embedding a Pascal engine in Rust, maybe a memory-mapped file; errors raise to the Rust container.) (Decided: handles (`Ptr<T>` over `Rc<RefCell>`), not a heap object; panics caught by `run()`; `docs/architecture.md` §5, §12, §17.)
  - [x] How should we represent arrays and strings in Pascal in Rust? (`PArray<T, LO, N>` and `PString`; §8, §10.)
  - [x] How should we represent pointers and the heap? (`Ptr<T>`; §12.)
  - [x] How should we represent records? (structs, variant records flattened; §11.)
  - [x] How do we build a FFI to Rust? (§22.)
  - [x] How do we build a FFI to C? (§22, tier 3.)
  - [x] Is it even possible to have a FFI to Pascal that's already been compiled? (Low priority; save ideas for later.) (Not directly; ideas in §22.)
  - [x] Can we skip implementing threads? Do Pascal / Turbo Pascal even have those built in? (Yes, skip; TP has none; §23.)
  - [x] Write all of the core architectural decisions to a file named docs/architecture.md.
- [x] Start building a battery of tests to verify we're compiling it correctly:
  - [x] Are there existing tests in other open source projects that we can use for inspiration? (Free Pascal `tests/`, p2c `examples/`, Pascal Validation Suite; `notes/p2c.md` §3-4.)
  - [x] There should be tests that test what Pascal constructs translate into what Rust constructs. (`tests/translate`, snapshotted.)
- [x] How do we build at least a small standard library, including things like printing to stdout? (`crates/pascrust_runtime`.)
  - [x] There should be tests that compile Pascal to Rust and then run the Rust to generate expected output. (`tests/run`, expected output from `fpc -Mtp`.)
- [x] Is there an official standard library for Pascal or Turbo Pascal that has an interface that we can match? (The TP `System` unit plus `Crt`/`Dos`/`Graph`; list in `notes/p2c.md` §5.)
- [x] If possible, copy all the code in ~/Dropbox/programming/turbo_pascal to an examples directory and get them working.
  - [x] Copy the code into `examples/`.
  - [x] Get them working. (All ten programs transpile, build without warnings and run; see Phases 4 and 5.)

## Phase 0: Scaffolding (half a day)

- [x] Review `notes/open_questions.md` with the user; record answers at the top of that file.
- [x] Create the Cargo workspace: `crates/pascrust` (bin + lib) and `crates/pascrust_runtime` (lib with features `crt`, `dos`, `graph`). Edition 2024.
- [x] Write `CLAUDE.md`: project goals, the "intermediate Rust with teaching comments" style rule, where decisions live, how to run tests, the rule that every new Pascal construct needs a translate test and a run test.
- [x] Add `.gitignore` entries for `out/` and `target/`; add `rustfmt.toml`; add a GitHub Actions workflow (fmt, clippy, test).
- [x] Add `tests/README.md` describing the three test layers from `docs/architecture.md` §21.
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

## Phase 2: Core language (tier 1 of `docs/language_spec.md`)

Each bullet: parser + sema + codegen + a translate test + a run test. Order
chosen so each step is testable with the previous ones.

- [x] Integer, real, boolean, char expressions with TP precedence and promotion rules; `div`/`mod`/`/`; `as` casts inserted by sema's type info. Decide and document `i16` wrapping (`docs/architecture.md` §7).
- [x] `var` declarations with `Default` initialization; assignment; `if`/`else` (dangling else); `while`; `repeat`; `for to/downto` over integers.
- [x] `writeln`/`write` formatting: `:w`, `:w:d`, TP default real format, booleans. Run tests compare byte for byte.
- [x] `readln`/`read` from stdin for integers, reals, chars, strings (TP's lenient parsing). `.in` files in run tests.
- [x] Procedures and functions: value params, `var` params as `&mut`, function result variable, `Exit`, recursion, `forward`. Call-site `.clone()` insertion for non-Copy value args.
- [x] Nested procedures: lifting analysis in sema (free variables, transitive through calls, including recursion), codegen of lifted `fn outer_inner(captured..., params...)`. Tests: recursive nested proc, nested proc modifying an outer local, two levels of nesting, nested proc capturing an outer `var` parameter.
- [x] Globals lifted into `pascal_main` and threaded to procedures (same analysis). Test: a procedure that reads a global and is called from another procedure.
- [x] Alias detection in sema: same lvalue root passed twice (`Swap(x, x)`, `F(a, a[1])`, global passed by `var` to a procedure that uses it); report a transpile error with the Pascal position. Tests for each.
  - [x] Alias *unification*: when every call passes the same outer variable for a `var` parameter that the callee also captures (`life.pas`'s `Pause`), use the parameter for both instead of rejecting the program.
  - [x] PUNT: Copy-in/copy-out for `Swap(a[i], a[j])` on arrays of `Copy` elements (currently rejected because both roots are `a`).
- [x] `PString`: runtime type (`[u8; 255]` + `len` + `cap`), `Index<i32>`, concatenation, comparison, `assign` with truncation, `pstr!` literal macro; string vs char coercions in sema. `Length`, `Copy`, `Pos`, `Delete`, `Insert`, `Concat`, `UpCase`, `Str`, `Val`.
- [x] Enumerated types and `Ordinal` trait (generated impl per enum; runtime impls for integers, `u8`, `bool`); `Ord`, `Chr`, `Succ`, `Pred`, `Inc`, `Dec`, `Low`, `High`; `for` over enums and chars.
  - [x] `Inc`/`Dec` on an enum variable: `Inc(c)` came out as `c += 1`, which doesn't compile (found 2026-10-31); now `c.inc()`, `c.dec_by(n)`.
- [x] Subranges as host types.
  - [x] PUNT: `--range-checks` flag.
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
- [x] PUNT: Methods that don't modify `Self` could take `&self` instead of `&mut self`.
- [x] `Break`/`Continue`; `FillChar`/`Move` on arrays of scalars; `ParamCount`/`ParamStr`; `SizeOf` for sized types; `{$N+}` float types (`Single`, `Double`, `Extended`).
  - [x] PUNT: `Comp`, the 8087's 64-bit integer type: `unknown type`.
- [x] Milestone: `examples/database/` (bstunit, founit, filesys) transpiles and the non-interactive paths run (with `Crt` stubbed to plain mode).

## Phase 4: Crt, then the text-mode examples

- [x] `crt` feature: `crossterm`-backed terminal with TP state (`TextAttr`, `WindMin`/`WindMax`), `ClrScr`, `ClrEol`, `GotoXY`, `WhereX`/`WhereY`, `TextColor`/`TextBackground`/`HighVideo`/`LowVideo`/`NormVideo`, `Window`, `InsLine`/`DelLine`, `TextMode`, `Delay`, `Sound`/`NoSound`, `KeyPressed`, `ReadKey` (with `#0` + scan codes for arrows/function keys), colour constants, `Blink`.
- [x] Route all `write` output through the runtime (raw-mode `\r\n`, CP437).
  - [x] PUNT: `Window` clipping and scrolling of output (currently `Window` only affects `ClrScr`/`GotoXY`/`WhereX`).
- [x] CP437 → Unicode output mapping; reverse mapping on input.
- [x] Plain mode when stdout is not a TTY or `PASCRUST_TERM=plain`: cursor ops are no-ops, `ReadKey` reads stdin bytes, `Delay` honours `PASCRUST_FAST`.
- [x] Scripted-input run tests: in plain mode `ReadKey` reads the test's `.in` file; compare stdout (`tests/run/crt_plain.pas`).
- [x] Get working, one at a time, fixing transpiler bugs as found: `gmscode.pas`, `paper.pas`, `magic_square.pas`, `life.pas`, `blkjack.pas`, `menu.pas` (needs `Dos.Exec`, which may fail gracefully), `airport/` (four units). All transpile and build warning-free; `magic_square`, `paper`, `airport4` and `database/filesys` verified with scripted sessions.
  - [x] PUNT: Turn those scripted sessions into run tests (`examples/` acceptance tests in CI).
  - [x] `examples/database/founit.pas` empties the master file on delete: `Fcopy` stops at `eof` because nothing seeks back to the start. Confirmed as a bug in the original program and fixed there (plus `BST.ShiftLocations`, since later records move up a position).
  - [x] `bstunit.pas`: deleting a node with two children copied the successor's links (`Root^ := TempPtr^`). Fixed in the example.
- [x] PUNT: CI builds every example; runs the non-interactive ones. Done: CI builds every example warning-free (`examples_tests.rs`). Not yet: running any of them.

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
  - [x] A local that clashed with a captured global renamed the global everywhere (`global_count`); now only inside the procedure with the clash (`Analysis::capture_names`).
  - [x] Hoisting a deref argument evaluated it before earlier arguments with side effects (`ShowV(Bump, head^.value)`); earlier arguments with calls go into temporaries first now.
  - [x] `x shl n` with `n` at least the type's width didn't compile (constant) or panicked (variable); TP gives 0. Now `checked_shl(n).unwrap_or(0)` unless the count is a literal below the width. A shift also has the type of its left operand now, as in TP.
  - [x] `p^.x := p^.x + 1` hoisted a temporary; it's `p.get_mut().x += 1` now, when the added part reads no pointer. (Places are compared in the Pascal, not in the rendered Rust.)
  - [x] `Random(n)`: checked against a disassembly of TP's System unit (github.com/daelsepara/turbo-pascal-assembly): it uses the full 32-bit seed, `(seed * n) shr 32`, as we do. `n` is now truncated to a `Word`, as TP's parameter is. Not done: `RandSeed` as an assignable variable.
  - [x] `{$I-}` and a real `IOResult`.
  - [x] `SeekEof`/`SeekEoln`.
  - [x] `LastMode` (done with the Crt variables).
  - [x] `Val` corner cases: a number that doesn't fit wraps with code 0, as in Free Pascal (TP left it undefined); a minus sign is an error for a Byte or Word; after an error the variable is 0.
  - [x] PUNT: Graph: should `SetBkColor` use the current `SetPalette` entry? Do XOR-mode rectangles lose their corners in the real BGI? `FExpand` ignores `ChDir`.
  - [x] `i16::MIN div -1` panicked (runtime error 255). TP doesn't wrap: its 16-bit divide traps, which it reported as runtime error 200, and so does PascRust now.
  - Not changed: `Round(2.5)` is 3. The reviewer suggested ties-to-even to match Free Pascal, but Turbo Pascal's documentation says halves round away from zero. `Eof` on a text file being written stays runtime error 104, as in Free Pascal (the reviewer thought TP returns True).

## Requests from the user, 2026-10-28

- [x] PUNT: Evaluation order where a function has side effects (the user: follow Free Pascal). Done: `Write(F, G)` writes F before running G; `n := n + Bump` reads `n` first. Not yet, and worth asking whether to chase: on aarch64 Free Pascal calls the function first for reals, strings and sets (`t := t + Ext` where `Ext` changes `t` gives `ayz`, PascRust `az`), and works out an assignment's target index before the right side (now done: such an index goes into a `let` first). These are Free Pascal code-generator details, and may differ on x86.
- [x] Rename the project to "PascRust" wherever a friendly name fits: the README, the other docs, `Cargo.toml` descriptions, the GitHub repo description, CLI help text and messages, comments and generated-code headers. The directory, the crates, the binary (`pascrust`), the repo name and other identifiers stay `pascrust`.

## Requests from the user, 2026-11-03

- [x] A Rust project that embeds Pascal code: how does its `cargo build` know to run PascRust on the Pascal? Done: `pascrust::build::library` in its `build.rs` and `pascrust_runtime::include_library!` (`examples/pascrust/rust_calls_pascal`, docs/embedding.md). (The user's question.) Likely answer: a `build.rs` build script, with PascRust as a build dependency, that translates the unit into Cargo's `OUT_DIR` (`cargo:rerun-if-changed=unit.pas` so a change in the Pascal is noticed) and an `include!` of it; a helper such as `pascrust::build::library("unit.pas")` would make that `build.rs` two lines. Document it in docs/embedding.md, and test it with the BSTUnit host.

- [x] Overflow (decided with the user, 2026-11-04): integer arithmetic keeps wrapping, as TP's default `{$Q-}` did; forbidding it would break correct programs (hashes, checksums, random number generators, `Byte` sums TP cut to fit). A library gets wrap-around only if the Rust program turns overflow checks off, so the docs give the `Cargo.toml` line and the runtime warns once when checks are on (part of the `build.rs` plan, `notes/build_rs.md`).
- [x] `{$Q+}` turns integer overflow into runtime error 215, as in TP: checked arithmetic in the regions it covers (the user, 2026-11-05: "if it isn't too hard").

## Requests from the user, 2026-11-02

In this order, after fixing `Inc`/`Dec` on enums and the Fable review of `076ffea..HEAD`:

- [x] PUNT: PascRust's own units, named with a `PR` prefix (`PRJson`, `PRTurtle`), as Turbo Pascal's types start with `T`. The user's choice (2026-11-02): the units are `PR...`, and their types follow TP's own convention inside them (`TJsonValue`, `PJsonValue = ^TJsonValue`), as `Dos`'s `SearchRec` does.
  - [x] `PRJson`: read and write JSON from Turbo Pascal (written in Pascal, the user's choice; JSON strings up to 255 characters).
  - [x] `PRTurtle`: turtle graphics on top of `Graph`, with Graph3's own routine names (the user's choice: `Forwd`, `Back`, `TurnLeft`, `TurnRight`, `SetHeading`, `Heading`, `SetPosition`, `Xcor`, `Ycor`, `Home`, `PenUp`, `PenDown`, `ShowTurtle`, `HideTurtle`, `Wrap`, `NoWrap`, `SetPenColor`), so a Graph3 turtle program ports by changing its `uses` line and its graphics setup.
    - [x] An example of an interesting turtle graphics program in `examples/` (`examples/pascrust/turtles.pas`: a Koch snowflake, a tree, a spiral) (the user's request, 2026-11-02): something that's fun to watch draw, such as a recursive tree or a Koch snowflake.
- [x] Document how Turbo Pascal code can call existing Rust code (and make it work: see the "FFI to Rust" item in Phase 8). (`docs/calling_rust.md`.)

## Requests from the user, 2026-10-29

- [x] PUNT: `Single`/`Double`/`Extended`/`Comp` on disk in their TP sizes (4/8/10/8 bytes; today all are 6-byte `Real`), in translated programs and `to-json` (the user, 2026-11-05: later; `to-json` refuses them meanwhile). See notes/file_json.md.
- [ ] An enum of more than 256 values: `tp_size` says 2 bytes (as TP), the runtime writes 1.
- [x] (Prioritized by the user, 2026-11-05; plan in notes/file_json.md: done as `pascrust to-json` and `from-json`, docs/data_files.md.) A command like `fmt` that turns a binary file written by a Turbo Pascal program (`file of T`) into JSON: an input file and an output file. A binary file doesn't say what its records look like, so it also needs the record type: probably the Pascal source that declares it and the type's name (`pascrust json founit.pas People mark.dat mark.json`), or the only `file of` type the program uses. It can reuse the runtime's TP-exact layout (`FileRecord`), decoding through the type's fields; one JSON object per record, in an array.

## Phase 5: Dos and Graph, then the graphics examples

- [x] `dos` feature (the parts the examples use; the rest is in Phase 7): `GetDate`/`GetTime` (`chrono`), `FExpand`/`FSearch`/`FSplit` with the drive-letter mapping and `\`→`/` translation, `Exec`/`DosExitCode` via `std::process::Command`, `ChDir`/`MkDir`/`RmDir`/`GetDir`, `GetEnv`, `DosVersion`, `FindFirst`/`FindNext`, `DiskFree`/`DiskSize` best effort; `PathStr` and friends as `PString` aliases.
- [x] `graph` feature, headless core first: 640x480 framebuffer of colour numbers, EGA palette (`SetBkColor`, `SetPalette`), `PutPixel`/`GetPixel`, Bresenham `Line`/`LineTo`/`LineRel`, `Rectangle`, `Bar`/`Bar3D` with 8x8 fill patterns (`SetFillStyle`/`SetFillPattern`/`GetFillPattern`), `Circle`/`Arc`/`Ellipse`/`FillEllipse`/`PieSlice`/`Sector`, `FloodFill`, line styles and `ThickWidth`, viewports (`SetViewPort`/`ClearViewPort`/`ClearDevice`), `SetWriteMode`.
  - [x] The rest of Graph: see Phase 7.
- [x] Bitmap text: the public-domain 8x8 VGA font (ASCII); `OutText`/`OutTextXY`/`SetTextJustify`/`SetTextStyle` (default font, integer scale; stroked fonts approximated with it)/`TextWidth`/`TextHeight`.
  - [x] PUNT: CP437 characters 128-255 in the font (box drawing, accented letters).
- [x] `InitGraph`/`Detect`/`DetectGraph`/`CloseGraph`/`GraphResult`/`GraphErrorMsg`/`SetGraphMode`/`GetGraphMode`/`RestoreCrtMode`/`GetMaxX`/`GetMaxY`/`GetMaxColor`/`GetAspectRatio`.
- [x] Tests: `PASCRUST_GRAPH=headless` draws without a window; `tests/run/graph_basics.pas` checks drawings with `GetPixel` (instead of golden images; see docs/architecture.md §19). `PASCRUST_GRAPH_SAVE=file.bmp` saves the screen for a look by eye.
- [x] `minifb` window backend: present the framebuffer, forward key events to `ReadKey`/`KeyPressed` while graphics mode is active, window serviced by the main thread (macOS needs that) while the program runs on its worker thread.
- [x] Get working: `clock/` (menu unit, clock unit, tester), `tictac/` (five units, `Ollie`, `CompUnit` AI). Checked headless with scripted keys and saved screenshots; the window opens on macOS.
  - [x] Transpiler fixes they needed: method call arguments that use the receiver (`obj.Prompt(obj.WhoseTurn)`) are computed first (`tests/run/method_args.pas`); two anonymous record types for same-named variables are `EndPoint` and `EndPoint2`, not `EndPoint_2` (`tests/run/anon_records.pas`).
  - [x] DOS drives: C: is the file system's root and other drives exist only when `PASCRUST_DRIVE_x` maps them (7d058ff; before, an unmapped drive was the directory the program started in) (`tictac` does `ChDir('a:\TicTac')` before every game), and names match files on disk case-insensitively.
  - [x] Play a whole game against the computer in the window, by hand (the user, 2026-11-06).
- [x] Milestone: every program in `examples/` transpiles, builds without warnings, and runs. Mark the original "Get them working" TODO done.

## Phase 6: Polish and readability

- [x] Have Claude Fable review every commit since its last code review (`git log 38138d3..HEAD`): three reviewers (language features, the rest of the transpiler, the runtime), every finding reproduced. Fixed:
  - Wrong output: virtual dispatch for `p^` passed to a `var` parameter of an ancestor type; `Random(n)` returned negative values for ranges over 32767 (it's a `Word`); `High`/`Low` of a subrange-typed variable; `FileMode := $40` opened files read-write; `GetDir` above drive C:'s root; printer output lost on Ctrl-C or a signal.
  - Didn't compile or panicked: `Output`/`Input`/`Lst` passed as `var Text`; a static method calling virtual ones, declared below the root, through a pointer; two `var` arguments in one heap record through an index (`arr[i]^.x, arr[i]^.y`; runtime error 204); a pointer and a field behind it as two `var` arguments; sets in `file of` records; `FillChar(b, SizeOf(b), True)`; `Swap($ABCD)`.
  - Missing checks: `New`/`Dispose` with a method that isn't a constructor/destructor; a `with` field as a `for` variable; descendant objects by value (now allowed, copying the ancestor part); `pascrust emit game.pas` without a directory; `SetTextBuf` made its file `mut` (a warning).
  - Tests: run tests and a new examples test now build generated code with `-D warnings`, so warnings fail the suite.
  - Decided with the user: a value outside a set's range is never in it, `300 in s` is False for a constant and a variable alike (FPC truncates the variable).
  - Open: `SizeOf` of a subrange variable (2 here, 1 in TP), of a small set (32 here), and of an object with virtual methods (TP adds 2); a `{$I-}` error still waiting isn't raised by later `{$I+}` I/O (documented in docs/architecture.md); `x shl 32` is 0 where TP masks the count to 5 bits.
- [x] PUNT: Open from the Fable reviews of the fix commits (2026-10-28). Done: a call in an index (`arr[Pick] := ...` where `Pick` reads `arr`, `items[Count]` in a method) is worked out into a `let` first, so it borrow-checks, and the target's index comes before the value, as in Free Pascal; `a[F] := a[F] + 1` runs F twice, as in TP. Still open: And: in `s := s + Bump` where `Bump` changes `s`, FPC calls `Bump` first and PascRust reads `s` first (TP's order unverified); `i in [-3..-1]` (set elements outside 0..255) depends on whether the ends are literals or named constants; `b in [True]` is `matches!(b, true)`, which clippy flags. Cosmetic: an enum value named like a runtime type gets a needless `_` (`Kind::Ptr_`); a long record's `/// Pascal:` line quotes the Rust name (`FileRecord_ = record ... end`). Two Pascal names that become the same Rust name in different scopes (a variable `enum_range` and a function `EnumRange`) clash.
- [x] Add an "Acknowledgements" section to the README thanking the projects we learned from and compared against. Our code is our own, but being able to run their tools and compare output helped a great deal: Free Pascal (`fpc -Mtp` produces most of our expected test output), daelsepara's turbo-pascal-assembly (the System unit disassembly behind `Random`), J R Stockton's Pascal pages, Borland's Turbo Pascal manuals, p2c (design ideas), and the Rust crates we build on (`clap`, `crossterm`, `minifb`, `insta`, ...). Also Claude Fable, for the code and design reviews.
- [x] `tpc`: a command-line compiler with Turbo Pascal's `TPC.EXE` options (`-$`, `-E`, `-U`, `-Q`; the rest accepted and ignored). Not an `fpc` look-alike, which would hide the real one.
- [x] PUNT: Under `{$I-}`, run each `Write`/`Writeln` argument as its own unchecked call, as TP does: in `WriteLn('x', IOResult)` with an error waiting, TP skips the `'x'` but prints the result (FPC prints `2`; we print nothing).
- [ ] Keep the generated Rust reasonably readable by a human. It's fine for it to call helper functions in the runtime (`pascrust_runtime`) when a Pascal construct needs support code; what matters is that a reader can follow the translated program.
  - [ ] `and`/`or`/`xor` of two Byte values compared with a constant that fits: `s.Attr and Directory = Directory` comes out as `i16::from(s.attr) & i16::from(DIRECTORY) == i16::from(DIRECTORY)`; `s.attr & DIRECTORY == DIRECTORY` computes the same (found by the Fable review of FindFirst).
- [ ] Read through the generated Rust for `life.pas` and `bstunit.pas` as if reviewing a colleague's code; fix anything an intermediate Rust programmer would not write (unneeded clones, casts, parentheses, `mut`).
- [x] PUNT: Commit showcase generated output for `hello.pas` and `life.pas` (per open question 7) with a refresh script.
- [x] Make transpiler diagnostics show the source line with a caret; add `--explain` text for the alias-detection error. (Instead of `--explain`, errors caused by a limit of Rust link to a worked rewrite in `docs/rust_limits.md`.)
- [x] PUNT: Write `docs/translating.md`: a user guide with the Pascal→Rust mapping table and the known limitations, generated partly from the translate-test snapshots.
- [x] PUNT: Measure and trim compile time of generated crates (feature flags, no unused dependencies).

## Phase 7: The rest of Turbo Pascal 7's standard units

Borland's *Turbo Pascal 7.0 Programmer's Reference* lists every routine,
variable and constant of the standard units, so "done" is well defined. The
order below puts first what old programs use most. Where the real thing
makes no sense on a modern machine, we **fake it**: accept the call and do
something harmless and plausible, so the program runs, and list the fake in
`docs/language_spec.md`. Each item gets translate and run tests, checked against
`fpc -Mtp` where FPC matches TP.

Next up (moved here from Phase 6 at the user's request)
- [x] Carry Pascal comments into the Rust output at statement boundaries: anchored to statements, declarations, record fields, procedures and block ends (`codegen/comments.rs`, docs/architecture.md §20).

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
- [x] Never: `Intr`, `MsDos`, `Keep` are a clear "needs real DOS" error (`UnitRoutine::refused`).
- [x] `GetIntVec`/`SetIntVec` faked: vectors are kept and given back, handlers (`interrupt` procedures, `@Handler`) never run, and installing one gives a warning (the user's call).

Graph, the rest
- [x] `DrawPoly`/`FillPoly` (arrays of `PointType`), `GetArcCoords`, and the `Get*Settings` records (`GetLineSettings`, `GetFillSettings`, `GetTextSettings`, `GetViewSettings`).
- [x] `GetImage`/`PutImage`/`ImageSize` with the `PutImage` modes, in TP's own buffer layout (step 4 of `notes/untyped_memory.md`).
- [x] Palettes: `GetPalette`, `GetDefaultPalette`, `SetAllPalette`, `SetRGBPalette`, `GetPaletteSize`; `GraphDefaults`; `SetAspectRatio`; `SetUserCharSize`; `GetDriverName`/`GetModeName`/`GetMaxMode`/`GetModeRange`.
- [x] `SetActivePage`/`SetVisualPage` (two framebuffers, for flicker-free animation).
- [x] Fake: `InstallUserFont` and `SetGraphBufSize` succeed and change nothing (there are no `.CHR` files to load). `RegisterBGIdriver`, `RegisterBGIfont` and `InstallUserDriver` are refused instead, with the reason: they take `@DriverProc` (code linked in from an `.OBJ` file) or a driver file, and the call can simply go.

Last: raw bytes and C strings
- [x] PUNT: From the Fable review of 8172e5e..afba062, left for later: a pointer chain that comes back to the same record (`p^.next^.next^.x := ... * 2` with a cycle) panics with error 204 on the write (the place keeps every pointer on the path borrowed; clone the last handle first, as `with p^` does). (Not worth chasing, the user says: `Val`'s error positions differ from Free Pascal's by one for a too-long number into a Byte or Word and for `'1e'`.) TP 7 may have wrapped `MinLongInt div -1` (its LongInt division is a software routine) where PascRust gives error 200.
- [ ] Fable code reviews: resumed (the user, 2026-10-30). Reviewed: everything up to 721ee55 (batches A-R). Not yet reviewed: `721ee55..HEAD`. Keep this range up to date.
- [ ] Findings of review batch R (b89b0fe..721ee55):
  - [x] `Inc(i, 32768)` on an Integer (`Inc(s, 128)` on a ShortInt) was an out-of-range literal that rustc refused; it wraps to -32768 now, as in TP.
  - [x] Division by a constant 0 in a subrange, array bound, string length, variant or case label gave a second, generic error (and a type declaration both twice); the docs didn't say it's a compile error, nor that a parameter's type must be a type's name.
  - [x] Real division by zero (`x / 0`) printed `+Inf`; TP stops with runtime error 200. The user (2026-11-06): stop with runtime error 200, as TP did.
  - [x] PUNT: `from-json` takes `[1e-400]` as 0 without an error (serde_json reads it as 0.0), where `[1e-320]` is refused as too small. The user (2026-11-06): leave it.
  - [x] Codegen's `constant_int` was a second constant evaluator next to sema's `eval_const`, and knew less (`Inc(b, Ord('A'))` under `{$Q+}` took the long form). Fable's advice (the user: "Try asking Fable"): sema records each constant expression's value (`Analysis::consts`) and codegen reads it; a named constant stays a name (`b += K as u8`), as a reader wants to see where 5 comes from.
  - [x] Also from that advice: `Ord` of a Char or char literal into an 8-bit type is written as the character (`b.checked_add(b'A')`, `b = c`), not `i16::from(b'A') as u8`; a set range of constants (`i in [K + 1..K * 2]`) becomes a `matches!` pattern; `case_labels` can be read from `consts`.
  - [x] PUNT: `Inc(c, s)` on a Char isn't checked under `{$Q+}`; a literal above MaxLongInt (`2147483648`) is accepted and rustc refuses it.
- [x] Findings of review batch Q (70d5d51..b89b0fe):
  - [x] `Dec(b, -K)` (a constant step) was error 215 under `{$Q+}`, where `Dec(b, -5)` turns round; a checked index was written twice in the read-then-write block (now only for a place through a pointer); `Inc(a^[i], a^[i])` shadowed `value` with `value`.
  - [x] Division by a constant 0 went to rustc, which refused it; it's TP's compile error now. `p: ^Node` as a parameter's type said "unknown type `Node`"; now it says to name the pointer type. `from-json` printed a Real out of range with hundreds of digits.
  - [x] Under `{$Q+}`, `Inc(b, s)` or `Dec(b, s)` on a Byte or Word with a negative variable `s` was error 215 (`b.checked_sub(s as u8)`). The user (2026-11-06): add in a wider signed type and check the result, as a negative constant step does.
- [x] Findings of review batch P (44cbc65..70d5d51):
  - [x] `Exec('true.exe')` ran `/usr/bin/true`: a bare name was looked up on `PATH`; now it runs from the current directory only, as in DOS.
  - [x] `{$Q+}` `Inc(r^.q^.N, 40)` (two pointers) didn't compile; `from-json` refused `to-json`'s output for a Boolean variant before a Byte one; the Real range in `from-json` was approximate (2.92e-39 stored as 0, the largest Real refused).
  - [x] `tpc -E` didn't make the directory, so the README's recipe for menu.pas's games failed; Crt didn't put its colours back after `Exec`.
  - [x] Under `{$Q+}`, `i * (2 + 3)` checked the constant `2 + 3`, and `w * (2 + 3)` (a Word), `l - (2 + 3)`, `Dec(b, 2 + 3)` didn't compile. The user: simplify, as Fable advised: a constant expression is never checked (TP works it out when it compiles), so it's written as under `{$Q-}`: `i.checked_mul(2 + 3)`.
  - [ ] From Fable's advice on that: a kept constant expression is computed in the context's type, where TP used 32 bits: `i * (30000 + 30000 - 40000)` fails with rustc's `overflowing_literals`, and `const C = 32767; i * (C + 1)` with `arithmetic_overflow` (both modes). Fold such an expression to its value when an intermediate doesn't fit. (Done: `Inc(b, 2 + 300)` wraps as `Inc(b, 302)` does, and `Dec(b, -K)` turns round as `Dec(b, -5)` does.)
  - [x] An anonymous pointer type as a record field (`q: ^Inner;` inside a record) panicked codegen ("only named pointer types are emitted"). The user: support it. It's `Ptr<Inner>`, as for a variable.
- [x] Findings of review batch O (f4219c0..44cbc65):
  - [x] `{$Q+}`: `(2 + 3) * i` didn't compile (a suffix on a folded expression); `Inc(p^.N)` was runtime error 204 (two borrows of the heap record).
  - [x] `to-json` read a Boolean nested tag as its byte, not as TP's any-nonzero-is-True, so `from-json` refused `to-json`'s own output.
  - [x] `to-json` on a source with `{$I file.inc}` gave a confusing error; `from-json` wrote 0 for a Real too small; `owners` moved into `Out` (no more `too_many_arguments`); README names `{$Q+}`.
  - [x] Tests: nested variant parts, a tagless record over a Boolean, an object with a parent, sets of 10..20 and of Byte, files of strings and integers, a unit as the source (tests/json/more.pas).
- [x] Findings of review batch N (c537266..f4219c0):
  - [x] `Halt` inside `call` with `panic = "abort"` aborted with "Halt(0)" instead of exiting with its code.
  - [x] The abort note's wording assumed `call`; `cast` lost its doc comment to `is_unsigned`; the test matched "Runtime error 2" in "201".
  - [x] `Byte(i shr 1)` and `Word(Integer(i shr 1))` kept a needless `as i16`. (Still open, cosmetic: `Word((i shr 1) shr 1)` has `as i16 as u16` on the inner shift.)
  - [ ] A typecast of a real, a string or an enum value out of range in a constant says "expected a constant expression" (and then "unknown identifier"); a specific message would be better (fpc: "Illegal type conversion").
  - [x] PUNT: `const X = Small(300)` for `type Small = 0..10` gives 44, unchecked; fpc reports a range error. TP 7 unverified (for the user, above).
- [x] Findings of review batch M (a7c84bf..c537266):
  - [x] With `panic = "abort"`, an error inside `call` or an I/O error under `{$I-}` ended the program with nothing on stderr.
  - [x] `Word(+5)` wasn't folded (`5 as u16`); `Word(Integer(-1) shr 1)` cast to `i16` and back.
  - [x] Tests: out-of-range and nested conversions, a folded negative literal as a method's receiver, `not Odd` of expressions (tests/run/casts_folded.pas).
  - [x] Typecasts in constant expressions (`const W = Word(-1)`, `case` labels, array bounds): TP 7 allows them (the user: now). `IntKind::wrap` is shared with `value_cast`.
  - [x] `Odd`/`not Odd` of a Byte/Word as `b.is_multiple_of(2)`, as clippy's default `manual_is_multiple_of` wants (the user's choice).
  - [x] A bit operator's operands get parentheses unless simple: `(i << 1) & 3`, `(a & b) | c` (the user: parentheses make it easier to understand).
- [x] Findings of review batch L (6b96dc9..a7c84bf), fixed (the user's choices: `super::` in linked Rust, no overflow probe under `panic = "abort"`, Boolean `Succ`/`Pred` as for an enum, clippy-clean where a person would write it so):
  - [x] A linked `.rs` that names the unit's types `crate::X` (as docs/calling_rust.md and calls_rust.rs say) doesn't compile under `include_library!`; `super::X` works everywhere.
  - [x] The overflow probe aborts the program under `panic = "abort"` (`catch_unwind` can't catch).
  - [x] A user cast of a negative literal (`Word(-1) div 2`) doesn't compile.
  - [x] `Succ`/`Pred` of a Boolean doesn't compile (`ok + 1`).
  - [x] The `FEATURE_CRT` check's message comes after the missing-name errors, not first (docs wording).
  - [x] `single_file` keeps the comment explaining the `#![allow]` it strips.
  - [x] Readability: `not Odd(a)` as `!(a % 2 != 0)`; a shift mixed with arithmetic without parentheses (clippy's `precedence`); `Char(66)` as `(66 as u8)`.
  - [x] docs/embedding.md doesn't mention `library_with`/`unit_dirs`, or that a linked `.rs` uses `super::`.
  - [x] `tpc_tests` need the `cli` feature (`required-features`).
  - [x] Low: `calls_rust.pas`'s sort has no run test of its own (left: `rust_callback` covers Rust calling back into Pascal, and the examples test builds it).
- [x] Findings of review batch K (1fc2d9b..6b96dc9), fixed:
  - [x] A function ending in `repeat ... until False` doesn't compile (`if false { break; }` makes the loop `()`); `repeat ... until False` should just be `loop`.
  - [x] A `goto` counts as leaving a `while True` loop, so the Rust after it is unreachable (warnings): only a `Break` leaves it.
  - [x] A labeled `Break` (`L: Break`) isn't seen: the loop is taken as endless and what follows is commented out (wrong output).
  - [x] `{$I-}` I/O in Pascal code called outside `call` prints "Runtime error 2" although the Pascal handles it.
  - [x] The `with` comment still appears when the rewrite only copies the pointer (`with p^ do p := q`); that copy deserves its own comment.
  - [x] A declared label no `goto` targets, after an endless loop, makes the next statement live (a warning). The user (2026-11-04): accept it, it's rare.
  - Decided (the user): a runtime error in Pascal code called outside `call` stays a Rust panic (exit 101); `call` gives TP's code.
  - [x] A program whose only runtime uses are dead gets an unused-import warning for the prelude.
  - [x] docs/architecture.md doesn't mention endless loops in the dead-code rules.
- [x] Findings of review batch J (bc55d07..1fc2d9b), fixed (the user's decisions: always a context; code after an endless loop is dead; the innermost `call` gets a `Halt`; `tpc` keeps the file-name check):
  - [x] `--lib`: an interface routine that assigns to a value parameter (`mut n`) makes a `lib.rs` that doesn't compile.
  - [x] `Halt` inside a `{$I-}` I/O statement inside `call` is taken for an I/O error (`IOResult = n`), and the Pascal code goes on.
  - [x] A runtime error outside `call` is reported only if a `call` or `run` came first (the hook is installed there).
  - [x] `--lib`: a unit with no variables never runs its initialization parts (no context, no `new()`).
  - [x] `while True do` with no way out but `Exit`: the trailing `result` and the statements after it are unreachable (warnings).
  - [x] A `repeat` body ending in dead code after a `Break`/`Exit` still gets the unreachable `until` test.
  - [x] Small: `library_method`'s unused `captures`; docs/embedding.md on errors outside `call` and on `Halt` in nested `call`s.
- [x] Findings of review batch I (a132173..bc55d07), fixed:
  - [x] A `goto` out of a goto loop nested in another goto loop (`Main:` ... `Inner:` ... `goto Finish` ... `goto Main`; `Finish:`) loops forever: the block isn't widened past the outer loop.
  - [x] `--lib` on a unit that uses Graph hangs in `InitGraph` (the window needs `run`'s main thread). The user's decision (2026-11-03): refuse it; a program that uses Graph should be a Pascal program that calls out to Rust.
  - [x] `--lib`: `use counter::*` in the host is ambiguous (the crate and its module have one name); the unit's own module should be private, re-exported.
  - [x] `--lib`: no public way to shut the runtime down (Crt's raw terminal, a `|command` printer).
  - [x] Two units linking the same Rust file is an error since bc55d07; only different files with one module name should be.
  - [x] A runtime error outside `call`, after a `call`, kills the host without a message (the hook stays installed).
  - [x] `Halt` inside `call` exits the host. Decided: `call` returns it as an `Err` instead.
  - [x] `Exit` or `Break` as the last statement of a `repeat` body: the `if cond { break; }` after it is unreachable (a warning).
  - [x] A labeled final `Exit` (`Fin: Exit;`) still warns; a dropped final `Exit` loses its comment.
  - [x] `--lib` with a file not named after its unit: the error quotes the made-up program's `uses` line.
  - [x] `--lib`: an interface `external` isn't re-exported; a unit's variables are parameters the host must own. The user's decision (2026-11-03): bundle a unit's variables into one generated record for `--lib` now.
  - [x] PRTurtle: `Wrap` in a zero-size `TurtleWindow` hangs.
  - [x] `while True do` gives rustc's `while_true` warning: it should be `loop`.
  - [x] `tpc unit.pas`: decided, check the unit compiles and say how units are used (the program that uses it, or `pascrust build --lib`), naming `tpc`.
  - [x] Small: `with r do goto L` emits the `with` comment for nothing.
- [x] Findings of review batch H (86ec036..a132173), fixed:
  - [x] A labeled main loop with a forward `goto` out of it (`Main: begin ... goto Finish ... goto Main end; Finish:`) loops forever: the loop is put outside the block.
  - [x] `Inc(i, -32768)` on an Integer (a negative step at the type's minimum) gives a literal out of range.
  - [x] Constant arithmetic that wraps (`b := 250; Inc(b, 10)`) is rustc's deny-by-default `arithmetic_overflow` error.
  - [x] PRJson: a `\u` escape above 32767 overflows an Integer and gives a garbage byte.
  - [x] PRJson: a string whose escaped form is over 255 characters is written without its closing quote.
  - [x] PRJson: `1e400` reads as infinity and is written as `+Inf`.
  - [x] Dead code after `if ... then goto A else goto B`, or after a `begin ... goto L end`, still warns.
  - [x] A routine whose last statement is `Exit` warns (unreachable `result`).
  - [x] Commented-out dead code loses its indentation.
  - [x] PRTurtle: `Wrap` with the turtle already outside the window draws a stray line; `Forwd(0)` there runs away.
  - [x] The `r#` branch in `link_externals` is dead (`snake_case` already escapes keywords); `{$L sub\type.rs}` with a backslash isn't found.
  - [x] Small: `goto::dead`'s doc comment; `Inc(b, 256)` emits `b += 0`; the comment names `sound_rs.rs` for a renamed module; two `{$L}` files with one name; PRTurtle leaves Graph's colour and write mode changed; PRJson's tiny numbers and `JsonSet`/`JsonAdd` on the wrong kind.
  - Kept as is (decided, open to the user): `Round` rounds half away from zero, as TP documented (FPC rounds half to even); `JsonToString` cuts at 255 characters (documented).
- [x] Findings of review batch G (076ffea..86ec036), fixed:
  - [x] A labeled `begin ... goto Main end` (a main loop) gets an unreachable `break 'main` after the `continue` (a warning).
  - [x] Two labels on one statement (`L1: L2: x`): a `goto L2` is refused as a jump into a statement.
  - [x] `Inc(p^.v, p^.n)` (a step read from the heap record being changed): runtime error 204.
  - [x] `Inc`/`Dec` on a Boolean doesn't compile.
  - [x] In a method, a bare procedural field called with arguments (`Show(n)`) is refused.
  - [x] `Dec(c, 2)` on a Char gives `c -= 2 as u8` (a needless cast).
  - [x] Document that `Inc`/`Dec`/`Succ`/`Pred` past the ends of an enum are a range error (a Rust enum can't hold an ordinal outside it; TP with `{$R-}` stored it).
  - [x] Dead code after an unconditional `goto` or `Exit` gave an `unreachable_code` warning. The user's choice: the Rust keeps it, commented out.
  - [x] Label a loop that holds a goto block, so `Break`/`Continue` inside the block work (the user: now).
- [x] Findings of review batch F (3ab3ee4..076ffea), fixed:
  - [x] A parameterless procedure field or element called as a statement (`H.S;`, `with H do S;`) is called twice: `.get()(..).get()(..)`.
  - [x] Every unary builtin (`Odd(F)`, `Abs(R.Next)`) refuses a parameterless function value; only `Assigned` should look at the value.
  - [x] In a method, a bare object field holding a parameterless function (`N := Next`) isn't called.
  - [x] `Val(S, p^.v, p^.code)`: two targets in one heap record are runtime error 204.
  - [x] `Insert`, `Delete`, `FillChar`, `Move` writing a heap field while reading the same record: runtime error 204.
  - [x] A call statement through an element or a dereference (`Arr[1];`, `PS^;`) is refused.
- [x] Findings of review batch E (d1097cb..3ab3ee4), fixed:
  - [x] Answered (a newtype, the user): a routine kept in a procedural type whose globals include a variable of that type (a state machine's `Current := Second`) makes a recursive type alias (rustc E0391). A newtype `struct State(ProcVar<...>)`, or a documented refusal?
  - [x] A parameterless function *variable* is called where TP uses its value: `G := F`, `Run(F)`, `Assigned(F)`, `@F`, `@F = nil`.
  - [x] The converse: a parameterless function in a field or element (`R.Next`, `Fs[1]`) isn't called in an expression; the `F = G` message is misleading there.
  - [x] A unit's procedural type whose routines use a program-declared type: the unit's module spells `Stats` without `crate::`.
  - [x] Two untyped `var` arguments from one heap record (`Both(pp^.y, pp^.x, 2)`) are runtime error 204.
  - [x] The typed-constant tag check doesn't reach nested variant parts.
  - [x] `Str(p^.v:2, p^.name)` and `Val` into a heap record's field from the same record: runtime error 204 (the value needs a `let`).
  - [x] Answered (216, the user): a nil procedural call is runtime error 210 (TP: "Object not initialized"); FPC and the design note say 216.
  - [x] TODO's "Procedural types and procedure parameters" item; a procedural type alias's doc comment shows the signature, not the declaration (kept: a record's shows its type alone too).
- [x] Findings of review batch D (697e1d1..d1097cb), fixed:
  - [x] A routine that changes a lent Char array through a `var` argument or as a method isn't run first (check_lends looks at captures only; use the call site's var_args too).
  - [x] A record or enum field in a tagless variant part doesn't compile (its type needs a FileRecord: register it); then a nested setter that reads its own record needs the value in a `let` first.
  - [x] Insert/Delete/FillChar on a `string[n]` field of a tagless part, and `Delete(p^, ...)`: `place()`'s leading `*` before a method; and `S4(data) := t + 'z'` needs parentheses in `write_string(&(...), n)`.
  - [x] An index inside a byte-field place is evaluated as a mutable place (`p.B[p.B[0]] := 9` borrows `p` twice).
  - [x] Sets of a subrange take 32 bytes in SizeOf, byte views and files; TP sizes them by range (`(Max div 8) - (Min div 8) + 1`).
  - [x] Move between two fields of one heap record of different types is still runtime error 204.
  - [x] A cast on the left of `<` doesn't compile (`i shr 1 < 50`, `Word(i) < w`).
  - [x] A `string[n]` field of a tagless part writes cap+1 bytes and clamps the length; TP touches Length+1 bytes (the `S[0] := #3` overlay idiom).
  - [x] Untyped var arguments skip the aliasing check (a rustc error instead of PascRust's message).
  - [x] Smaller: `Str(1234:5, s)` into a `string[3]` isn't cut; `New(PNode(q))` through a Pointer; a typed constant whose tag selects another variant than its fields; the accessor comment's "bytes 0 to 4".
- [x] Untyped memory, as planned in `notes/untyped_memory.md` (agreed with Fable's review; the user said go ahead). Done: step 0 (TP-exact typed files), step 1 (untyped `var` parameters, byte views, `Move`/`FillChar` between types), step 2 (untyped files, `BlockRead`/`BlockWrite`), step 3 (the untyped `Pointer`, `GetMem`/`FreeMem`), step 4 (`GetImage`/`PutImage` in TP's layout), step 5 (`PChar` and `Strings`, with `Char` arrays lent a copy for one statement).
- [x] Question for the user (from a Fable review): is the directory the program starts in a drive's root? Answered by 7d058ff: no, C: is the file system's root, so the start directory is just the current directory, and `FindFirst` lists `.`/`..` there.
- [x] Untyped files (`file` without `of`) with `BlockRead`/`BlockWrite`, and untyped `var` parameters, as byte views of records and arrays (with `SizeOf`).
- [x] `PChar` and the `Strings` unit, as a byte buffer plus an offset; a `Char` array used as a `PChar` is lent a copy for the statement, and keeping a `PChar` into one is a compile error (the user chose this over shared arrays, 2026-10-29).
- [x] `WinDos`: the `Dos` routines, and wrappers with `PChar` strings.
- [x] `Write`/`Read`/`Str`/`Val`/`Assign`/`Rename` on `Char` arrays and `PChar`s (TP 7's extended syntax).
- [x] Packed strings: a string constant assigned to a `Char` array, and one used as a string (assigned, compared, joined, passed).
- [x] Skipped, on purpose: `Graph3` and `Turbo3`, TP 7's units for running Turbo Pascal 3.0 programs (TP 3's turtle and point graphics, and its `Kbd` file, integer `MemAvail` and the like). The examples are TP 5.5/6.0 code; `uses Graph3` or `Turbo3` says it's a TP 3.0 compatibility unit PascRust doesn't provide. Graph3 on top of our Graph was estimated (2026-10-31: CGA modes and palettes, `Write` on the graphics screen, turtle graphics, no FPC to compare with) and punted by the user until a program needs it.

## Phase 8: Tier 3 and FFI (as interest dictates)

- [x] Variant records and procedural types, as in `notes/variant_records_and_procedural_types.md`: step 1 (tagged variant parts, flattened, with TP's layout in files), step 2 (tagless parts as TP's bytes, behind accessors), step 3 (nested parts), steps 4-6 (procedural types: values, calls, globals through them, as bytes).
- [x] PUNT: Left for later, if a program needs it: `Func(P)`, a `Pointer` turned back into a routine.
- [x] PUNT: Argument order in `Inc(a[Pick], Bump)`: PascRust runs `Pick` before `Bump`, Free Pascal `Bump` first (from the Fable review of 99e4f09). TP doesn't document it; obscure.
- [ ] Hex constants as TP reads them: a 32-bit `$AABBCCDD` is a `LongInt` bit pattern (`$FFFFFFFF` is -1); PascRust says it's out of range.
- [ ] An anonymous record type is named `Record` in Rust (`fn pair(&self) -> Record`); naming it after its field (`Pair`) would read better.
- [x] Field names needn't avoid the runtime's function names (done: `names::field_name` escapes only Rust keywords): a field `W` becomes `w_` because `w` is the runtime's width helper, but a struct field can't clash with a function.
- [x] Virtual methods, constructors, destructors: a trait per object in the hierarchy for dispatch, an enum per hierarchy for pointers (`notes/virtual_methods.md`, docs/architecture.md §16), and `TypeOf`.
- [x] Procedural types and procedure parameters (steps 4-6 of the variant records and procedural types item).
- [x] Structured `goto`, the user's idea (2026-10-30): a jump forward is a labeled block (`'done: { ... break 'done; }`), a jump back a labeled loop (`continue 'again`), in `sema/goto.rs`. Jumps into a statement, and crossing ones, are refused with a rust_limits link.
- [x] `Break`/`Continue` inside the statements a `goto` skips or repeats, for a loop around them: label that loop (`break 'while_loop`) instead of refusing.
- [x] TP-exact binary file layout including 6-byte `Real`, so `examples/database/mark.dat` can be read (step 0 of `notes/untyped_memory.md`).
- [x] PUNT: The per-procedure "place" option for `var` parameters that hit `RefCell` aliasing.
- [x] PUNT: Bundled-globals option (`struct Globals` per unit) for programs with very many globals. (By hand, a unit can do it already: `PRTurtle` keeps its state in one record, `TTurtleContext`.)
- [x] FFI to Rust: `external` routines in a Rust file named by `{$L file.rs}` (the user's choice, as TP linked `.OBJ` files), with crates from `//! cargo:` lines; `tests/run/rust_externals.pas` is the worked example.
- [x] Embedding: `pascrust build --lib unit.pas` and `pascrust_runtime::call(...) -> Result`, with a host example that calls `BSTUnit` from Rust (`tests/embed/bst_host.rs`, docs/embedding.md). Rust calling back into a Pascal program through a procedural value works too (`tests/run/rust_callback.pas`).
- [x] PUNT: FFI to C: `external 'C'` → `extern "C"` block with `#[repr(C)]` records, `build.rs` with `cc`.
- [x] PUNT: Ideas only, for already-compiled Pascal: Free Pascal `{$mode tp}` C-ABI library, or DOSBox bridge. Not scheduled.
- [x] PUNT: `--integer-width 32` flag if the user wants it.

## Testing track (runs alongside every phase)

- [x] PUNT: One translate snapshot and one run test per construct in `docs/language_spec.md` tier 1 and 2; keep a checklist table in `tests/README.md`.
- [x] If FPC is installed (open question 2): script `tests/gen_expected.sh` that compiles each `tests/run/*.pas` with `fpc -Mtp` and records its stdout as `*.out`; commit the outputs.
- [x] Write our own self-checking run tests (exit code 0 when every check passes) for the System, Crt, Dos and Graph routines (`system_*`, `crt_vars`, `dos_misc`, `graph_*` and others; see `tests/README.md`), covering the kinds of cases Free Pascal's test suite covers. Don't copy FPC's or p2c's tests: both are GPL and this project is MIT. Running `fpc -Mtp` as an external program to produce expected output (`tests/gen_expected.sh`) is fine.
- [x] Negative tests: malformed programs produce the expected diagnostic text (snapshot). (`tests/errors`.)
- [x] Alias-detection tests (should fail to transpile) and `RefCell` aliasing tests (should fail at run time with the clear message). (`tests/errors/alias_*` and others; at run time, the runtime's `record_used_through_two_references_is_an_error` test.)
- [x] `cargo clippy -- -D warnings` on the transpiler and the runtime; `#![deny(warnings)]` is not applied to generated code, but generated code must build warning-free for every example. (CI and `out/gate.sh`; the run and examples tests build with `-D warnings`.)
