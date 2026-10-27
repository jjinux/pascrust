# Open questions for the user

Each question would change the plan if answered differently from the default.
The default is what `TODO.md` and `architecture.md` assume; answering "yes,
go with the default" needs no further action.

1. **16-bit `Integer`?** The plan maps `Integer` to `i16` with wrapping
   arithmetic (`overflow-checks = false`), because the examples were written
   for TP's 16-bit integers (`MaxInt = 32767`, `hi(WindMax)`, record layouts).
   Alternative: `i32`, which makes modern-feeling programs easier and
   overflow rarer but changes `MaxInt`, `SizeOf` and `file of record` sizes.
   **Default: `i16`, with a `--integer-width 32` flag later if wanted.**

2. **Install Free Pascal for reference outputs?** `brew install fpc` gives a
   TP-compatible compiler (`fpc -Mtp`) on this Mac. With it, expected outputs
   for execution tests can be generated and checked instead of hand-derived,
   and FPC's own `tests/test/*.pp` programs can be run both ways. Without it,
   every expected output is written by hand from the TP manual.
   **Default: yes, install it; keep the generated `.out` files in git so tests
   do not depend on FPC being present.**

3. **Binary compatibility with the shipped data files?** `examples/database/`
   ships `mark.dat` and `mark.key` written by TP 5.5 (`file of People` with a
   `string[30]`, a `LongInt` and a 6-byte `Real`). Reading them needs the
   exact TP record layout including the 6-byte software `Real` format.
   **Default: not initially; typed files use a simple fixed-size layout, and
   TP-exact layout is a tier-3 item. The database example will create fresh
   files.**

4. **Graphics backend.** The plan uses `minifb` (a software framebuffer in a
   window, 640x480, EGA palette, bitmap font) for the BGI `Graph` unit, plus a
   headless PNG backend for tests. Alternatives: `pixels`+`winit`, SDL2
   bindings, or a terminal-only stub.
   **Default: `minifb`, behind a Cargo feature so text programs never build
   it.**

5. **DOS paths and drives.** The examples hard-code `a:\tictac`,
   `c:\tp\bin\turbo.exe`, `a:;c:;h:`. Proposal: the runtime reads an optional
   `pascrust.toml` (or `PASCRUST_DRIVE_A=...` env vars) mapping drive letters
   to directories, converts `\` to `/`, and treats an unmapped drive as the
   current directory.
   **Default: that mapping scheme, with every drive defaulting to the current
   directory.**

6. **`Delay`, `Sound`, `Exec`.** Honour `Delay(ms)` literally (the examples
   use screen-saver and animation timing), make `Sound`/`NoSound` ring the
   terminal bell once (or no-op), and let `Exec` try `std::process::Command`
   and set `DosError` on failure.
   **Default: as described; a `PASCRUST_FAST=1` env var makes `Delay` a
   no-op for tests.**

7. **Where does generated code live, and is it committed?** Proposal:
   `out/<program>/` as a Cargo project per program, git-ignored, regenerated
   by `pascrust build`. Committing one or two generated examples under
   `examples/generated/` would let readers see the output without running
   anything.
   **Default: `out/` ignored; commit the generated Rust for `hello.pas` and
   `life.pas` as showcase, refreshed by a script.**

8. **Identifier style in generated code.** Convert to Rust conventions
   (`snake_case` functions and fields, `CamelCase` types) or keep Pascal
   spelling with `#![allow(non_snake_case)]`? Converting reads like Rust;
   keeping makes the Rust easy to match against the Pascal line by line.
   **Default: convert, and put the original Pascal name in a `///` comment on
   each declaration.**

9. **How strict should the type checker be?** TP rejects many things rustc
   would also reject, but rustc's errors would point at generated code. A full
   TP type checker is a big piece of work.
   **Default: check what codegen needs (types of every expression, lvalues,
   parameter kinds, alias detection) and report those errors with Pascal
   positions; rely on rustc for anything else, and treat any rustc error on
   generated code as a Pascrust bug to fix.**

10. **Scope of OOP.** The examples use static `object` methods and single
    inheritance only. Should virtual methods, constructors and destructors be
    in the plan at all?
    **Default: tier 3, designed (trait per object type) but not scheduled.**

11. **Interactive examples in CI.** `life.pas`, `blkjack.pas`, the menus and
    the games read keys in loops. Options: (a) only build them in CI, (b) add
    a scripted-input mode (`*.keys` file fed to `ReadKey`) and record
    golden screens.
    **Default: build all examples in CI; run the non-interactive ones;
    add scripted-input golden tests for a few interactive ones later.**

12. **Comment preservation.** Carrying the Pascal comments into the Rust
    output (as p2c does) makes the output much friendlier but needs comment
    tokens threaded through the parser.
    **Default: tier 3; the `///` declaration comments with the original
    Pascal headers come first.**

13. **Rust edition and toolchain.** Generated crates and the workspace use
    edition 2024 and the installed rustc 1.98; no MSRV promise.
    **Default: as stated.**

14. **Name of the generated entry point and runtime macros.** The plan uses
    `pascal_main()`, `pwrite!`/`pwriteln!`/`preadln!`, `w(x, n)`/`wd(x, n, d)`
    for formatting, and `pstr!("...")` for literals. Any naming preference
    (`pas::writeln(...)` function style instead of macros, for instance)
    should be settled before codegen is written.
    **Default: the macro names above.**

15. **Should `Exit` inside a nested procedure or the program block work the
    TP way?** `Exit` in the main block ends the program; in a nested procedure
    it returns from that nested procedure only. Both are natural with the
    lifted-function design.
    **Default: yes, TP semantics.**

## Answers (2026-10-27)

- Q1: `i16` with wrapping — confirmed.
- Q2: install Free Pascal and commit generated reference outputs — confirmed.
- Q3: no TP-exact binary layout initially — confirmed.
- Q8: Rust-style identifiers with the Pascal name in `///` comments — confirmed.
- All other questions: going with the defaults unless the user says otherwise.
