![Ruscal: a Rust-powered time machine for Turbo Pascal](docs/images/ruscal.png)

Ruscal
========

[![CI](https://github.com/jjinux/ruscal/actions/workflows/ci.yml/badge.svg)](https://github.com/jjinux/ruscal/actions/workflows/ci.yml)

Ruscal is a fairly-complete and well-tested Turbo Pascal to Rust
transpiler, so that you can more easily run your old Turbo Pascal programs
on modern computers.

Ruscal writes a small Cargo project for each program so you can build and run
your old Turbo Pascal code natively.

 * You can continue editing the Turbo Pascal code and transpile it every time.
 * You can transpile your Turbo Pascal code to Rust once and then edit the Rust going forward.
 * You can run your old Turbo Pascal programs even if they use graphics, including turtle graphics.
 * You can call out to Rust from a Turbo Pascal program.
 * You can call into Turbo Pascal from a Rust program.
 * You can read old binary data files written by Turbo Pascal.
 * You can also translate those files to and from JSON.
 * There's even a pretty printer for reformatting your Turbo Pascal code.

Ruscal was written by Claude Code Opus with help from Claude Code Fable and
JJ Behrens.

## Inspiration

My first programming language was Borland's Turbo Pascal running in DOS or
Windows 3.1. I've always thought that Pascal was the Python of its day in that
it was a really pleasant language to code in and was great for newbies. I had
all these Turbo Pascal programs that I had written as a student, but I had no
easy way to run them. I'm really inspired by the idea of creating a native
compiler, but there's hardly a point now that LLVM exists. I also wanted a
chance to play with Rust a bit more and to use Claude. Hence, my project to
transpile Turbo Pascal code to Rust. If you have some old Turbo Pascal code
lying around, give it a try!

## Usage

You need a Rust toolchain (`rustup`, stable). From the repository root:

```sh
# Transpile, build and run a program. The Cargo project goes in out/<name>/.
cargo run -p ruscal -- run examples/life.pas

# Arguments after `--` go to the Pascal program (ParamStr / ParamCount).
cargo run -p ruscal -- run myprog.pas -- first second

# Just print the generated Rust.
cargo run -p ruscal -- emit examples/paper.pas

# Transpile and build, without running. `-o` picks the project directory.
cargo run -p ruscal -- build examples/magic_square.pas -o /tmp/magic
```

Old sources with DOS (`\r\n`) line endings are fine everywhere: the
transpiler, `Readln` on standard input and text files, and `fmt` (which
writes UNIX line endings).

Or install the tools once with `cargo install --path crates/ruscal`, which
puts `ruscal` and `tpc` (below) on your `PATH`, and use
`ruscal run foo.pas`.

### Formatting Pascal sources: `ruscal fmt`

`fmt` tidies up old sources without changing what they mean. It re-indents
by the block structure (3 spaces a level, with a routine's `var` section,
nested routines and `begin ... end` one level in from its heading),
normalizes spacing (`x := y`, `a, b`, `name: string`), lowercases keywords,
and spells each identifier the way it's spelled most often. Line breaks,
strings and comments stay as they are, columns you lined up by hand stay
lined up, and DOS line endings become UNIX ones.

```sh
# Format files in place, printing the ones that changed. Give a program and
# its units together, so each name is spelled the same in all of them.
cargo run -p ruscal -- fmt examples/database/*.pas

# Only check: list the files that would change, and fail if there are any
# (handy in CI; this repository's tests check examples/ this way).
cargo run -p ruscal -- fmt --check examples/*.pas examples/*/*.pas
```

### Data files as JSON: `ruscal to-json` and `from-json`

A typed file (`file of T`) written by a Turbo Pascal program is just its
records' bytes. `to-json` reads them as a type the Pascal source declares,
and prints them as JSON; `from-json` writes JSON back as the binary file
(see [Data files as JSON](docs/data_files.md)):

```sh
# The user's 1997 database file: founit.pas declares its record type.
cargo run -p ruscal -- to-json examples/database/founit.pas examples/database/mark.dat
# Say which type when the source has several; -o writes to a file.
cargo run -p ruscal -- to-json tests/json/records.pas tests/json/items.dat --type Item -o items.json
# And back, after editing items.json.
cargo run -p ruscal -- from-json tests/json/records.pas items.json --type Item -o items.dat
```

### A Turbo Pascal style compiler: `tpc`

`tpc` takes the command line of Turbo Pascal's `TPC.EXE`, so old build
scripts and habits carry over. `tpc game` compiles `game.pas` (the extension
is optional, and names match ignoring case, as in DOS) and writes the
executable `game` next to it, as `TPC GAME` wrote `GAME.EXE`. The Cargo
project is built in a cache directory (`~/.cache/ruscal/tpc/`), so only the
program appears beside the source.

```sh
cargo install --path crates/ruscal      # once; or use target/debug/tpc

tpc life                                  # life.pas -> ./life
tpc -Ebin '-Uunits;lib' game              # executable in bin/, units also in units/ and lib/
tpc -Q '-$I-' backup                      # quiet, and I/O errors go to IOResult
```

| Option | Meaning |
| --- | --- |
| `'-$<switches>'` | Compiler switches, as at the top of each file: `'-$I-'`, `'-$R+,I-'`. Quote them in a Unix shell, which would read `$I` as a variable. |
| `-E<dir>` | Where to write the executable (the directory is made if it's missing). |
| `-U<dirs>` | More directories to find units in, separated by `;` (quote the argument in a Unix shell). |
| `-Q` | Quiet: no messages unless something goes wrong. |
| `-B`, `-M` | Build all / make: accepted; units are always compiled with the program. |
| `-D`, `-F`, `-G…`, `-I`, `-L`, `-O`, `-T`, `-V` | Accepted and ignored, with a note. They were about conditional defines, DOS memory, map files and the IDE's debugger. |

Options start with `-`. TP also took `/`, but on Unix a `/` starts a path.

Units named in a `uses` clause are looked up (case-insensitively) as
`<unit>.pas` next to the program, so a multi-file program runs from its main
file. The standard units `Crt`, `Dos` and `Graph` are built into the runtime.
`Graph` programs open a 640x480 window, and keys typed into it go to
`ReadKey`.

### Environment variables

| Variable | Effect |
| --- | --- |
| `RUSCAL_TERM=plain` | Don't take over the terminal for `Crt`; `ReadKey` reads standard input. This is automatic when stdin or stdout isn't a terminal, so you can script a program: `printf 'y\nq' \| ruscal run game.pas`. |
| `RUSCAL_FAST=1` | Skip `Delay`. |
| `RUSCAL_GRAPH=headless` | Draw `Graph` output without opening a window (for tests; `ReadKey` then reads standard input). |
| `RUSCAL_GRAPH_SAVE=screen.bmp` | Save the graphics screen to a BMP file at `CloseGraph` or `Halt`. |
| `RUSCAL_DRIVE_C=/some/dir` | Where the DOS drive `C:` lives for file names like `C:\DATA\X.DAT`. Without it, `C:` is the file system's root, `/`, so `C:\DATA\X.DAT` is `/DATA/X.DAT` and `GetDir` shows where you really are. Any other drive letter (`RUSCAL_DRIVE_A=/Volumes/USB` for a stick standing in for a floppy) exists only if you set it; a file name on an unset drive isn't found. Names match files on disk ignoring case, `A:X` is relative to the current directory when `ChDir` went somewhere on drive A, and `..` stops at the drive's root, as in DOS. A file name without a drive is relative to the current directory. |
| `RUSCAL_PRINTER=...` | Where the `Printer` unit's `Lst` prints: a file name (default `printer.txt`), or `\|command` to pipe to a command such as `\|lp`. See [Printing](docs/printing.md). |

### More documentation

- [Language spec](docs/language_spec.md): exactly which Turbo Pascal
  constructs are supported, and how closely.
- [Architecture](docs/architecture.md): how Ruscal works, and why it was
  designed that way.
- [Rewriting code that Rust can't express](docs/rust_limits.md): the few
  things that are fine in Pascal but break a rule of Rust (two `var`
  arguments into one array, ...), and how to rewrite them. Those errors
  link there.
- [Printing](docs/printing.md): where the `Printer` unit's `Lst` output goes.
- [Data files as JSON](docs/data_files.md): `ruscal to-json` reads a
  `file of T` written by a Turbo Pascal program, and `from-json` writes
  one back.
- [Timer interrupts](docs/timer_interrupts.md): getting a steady tick in a
  game without the timer interrupt, which Ruscal doesn't have.
- [Calling Rust from Turbo Pascal](docs/calling_rust.md): `external`
  routines written in Rust, linked with `{$L file.rs}` as TP linked
  assembly, and crates from crates.io.
- [Calling Turbo Pascal from Rust](docs/embedding.md): a Rust project's
  `build.rs` translates a unit as part of `cargo build` (or `ruscal
  build --lib unit.pas` makes a library of it), and
  `ruscal_runtime::call` turns its runtime errors into `Err`s.
- Ruscal's own units, whose names start with `PR`, written in Turbo
  Pascal and translated with the program that uses them:
  [PRJson](docs/pr_json.md), reading and writing JSON, and
  [PRTurtle](docs/pr_turtle.md), turtle graphics on top of `Graph`, with
  Turbo Pascal 3.0's turtle routines ([an example](examples/ruscal/turtles.pas)).

### What's supported

Most of Turbo Pascal 5.5/6.0 as students used it: the usual types, strings,
arrays, records (variant records too), enums, sets, pointers, procedural
types, nested procedures, `with`, units,
objects with inheritance and virtual methods (constructors, destructors,
`inherited`, polymorphic pointers, `TypeOf`), `goto` forward or back
within a statement list (as Rust's labeled blocks and loops), text and
typed files with
`{$I-}`/`IOResult`, `{$Q+}` overflow checks, `PChar` strings, and the `Crt`, `Dos`, `Graph`,
`Printer`, `Overlay`, `Strings` and `WinDos` units, plus Ruscal's own
`PRJson` (JSON) and `PRTurtle` (turtle graphics on top of `Graph`; see
[the turtle example](examples/ruscal/turtles.pas)). Turbo Pascal code
can also call Rust: `external` routines written in Rust, linked with
`{$L file.rs}` (see [Calling Rust](docs/calling_rust.md)); and Rust can
call Turbo Pascal: a Rust project's `build.rs` translates a unit while
`cargo build` runs, or `ruscal build --lib` makes a library of one (see
[Calling Turbo Pascal from Rust](docs/embedding.md)). There's an example
of each in `examples/ruscal/`. Some things a modern
machine doesn't have are faked so old programs still run: `Lst` prints to
a file or a command (see [Printing](docs/printing.md)), overlays need no `.OVR`
file, `MemAvail` reports plenty of memory, and interrupt vectors keep the
handlers set in them (which never run).

Not supported yet: a `goto` into a statement it isn't in, pointer
arithmetic other than on `PChar`,
keeping a `PChar` into a `Char` array (`P := A`; a `PChar` from `StrNew`
or `GetMem` works), and interrupts (`SetIntVec` keeps a handler, with a
warning, but never calls it). See the [language spec](docs/language_spec.md)
for the details and [`TODO.md`](TODO.md) for what's next. Runtime errors
print Turbo Pascal's message and exit code (e.g. `Runtime error 201: Range check error ...`).

## Examples

`examples/` holds the Turbo Pascal programs this project was built to run,
written in 1996-97.
Run them from the repository root:

```sh
# Text-mode programs (Crt)
cargo run -p ruscal -- run examples/blkjack.pas        # Blackjack
cargo run -p ruscal -- run examples/paper.pas          # Paper, scissors, rock
cargo run -p ruscal -- run examples/life.pas           # The Game of Life
cargo run -p ruscal -- run examples/magic_square.pas   # Solves a 4x4 magic square puzzle
cargo run -p ruscal -- run examples/gmscode.pas        # Encodes what you type as you type it
# A DOS menu, whose games ran from A:\SHANNON: build them there with tpc,
# then map drive A: to out/a. (Exec of GAME.EXE runs the program `game`.)
for g in life gmscode paper blkjack airport/airport4; do cargo run -p ruscal --bin tpc -- -Eout/a/shannon examples/$g.pas; done
RUSCAL_DRIVE_A=out/a cargo run -p ruscal -- run examples/menu.pas

# Multi-file programs: run the main file, and its units come along
cargo run -p ruscal -- run examples/airport/airport4.pas   # An airport runway simulation

# A record filing system. Drive C: is mapped to its directory, so when it
# asks which file to open, type `mark` to load the sample data (mark.dat, mark.key).
RUSCAL_DRIVE_C=examples/database cargo run -p ruscal -- run examples/database/filesys.pas

# Graphics (Graph): these open a window
cargo run -p ruscal -- run examples/clock/tester.pas   # A clock whose hands run backwards
# Tic-tac-toe expects to live in A:\TICTAC, so map drive A: to examples/.
RUSCAL_DRIVE_A=examples cargo run -p ruscal -- run examples/tictac/tictac.pas
```

`examples/ruscal/` holds examples written for Ruscal:

```sh
# Turtle graphics with PRTurtle: a Koch snowflake, a tree and a spiral
cargo run -p ruscal -- run examples/ruscal/turtles.pas

# Turbo Pascal calling Rust: calls_rust.pas links calls_rust.rs with {$L},
# and the Rust calls a Pascal function back to sort
cargo run -p ruscal -- run examples/ruscal/calls_rust.pas

# Rust calling Turbo Pascal: a Rust project whose build.rs translates
# examples/database/bstunit.pas (from 1997) as part of `cargo build`, then
# uses its binary search tree: it inserts four IDs, searches for one and
# deletes one, printing each result (src/main.rs is the Rust side)
cargo run --manifest-path examples/ruscal/rust_calls_pascal/Cargo.toml
```

The environment variable goes before `cargo`, so it applies to the whole
command (and to the program it runs).

## Developing

See [`CLAUDE.md`](CLAUDE.md) for the layout and commands, the
[architecture](docs/architecture.md) for the design, and run `cargo test`
for the whole test suite.

## Acknowledgements

Ruscal's code is its own, but we leaned on a lot of other people's work
while writing it. Thank you to:

- [Free Pascal](https://www.freepascal.org/). Running `fpc -Mtp` on our
  test programs gives us most of their expected output, so we can check
  that a translated program behaves as Turbo Pascal would.
- [daelsepara/turbo-pascal-assembly](https://github.com/daelsepara/turbo-pascal-assembly),
  an annotated disassembly of Turbo Pascal's System unit. It's how we know
  exactly how `Random` works.
- J R Stockton's
  [Borland Pascal/Delphi Random](https://people.cs.nycu.edu.tw/~tsaiwn/sisc/runtime_error_200_div_by_0/www.merlyn.demon.co.uk/pas-rand.htm)
  page, and his other Pascal notes.
- Borland, for the Turbo Pascal 7.0 manuals, which are still the best
  description of the language (see the References in the
  [language spec](docs/language_spec.md)).
- Dave Gillespie's p2c, the classic Pascal to C translator, for design
  ideas (`notes/p2c.md`).
- [font8x8](https://github.com/dhepper/font8x8) (public domain), the 8x8
  font `Graph` draws text with.
- The Rust crates we build on: [clap](https://crates.io/crates/clap),
  [crossterm](https://crates.io/crates/crossterm),
  [minifb](https://crates.io/crates/minifb),
  [chrono](https://crates.io/crates/chrono) and
  [insta](https://crates.io/crates/insta).
- Claude Fable, for the code and design reviews.
