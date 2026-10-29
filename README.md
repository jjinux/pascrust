PascRust
========

[![CI](https://github.com/jjinux/pascrust/actions/workflows/ci.yml/badge.svg)](https://github.com/jjinux/pascrust/actions/workflows/ci.yml)

PascRust is a Turbo Pascal to Rust transpiler, so that you can more easily
run your old Turbo Pascal programs on modern computers. It writes a small
Cargo project for each program, so you can build and run your old Pascal code
natively, and read the Rust it turned into.

PascRust was written by Claude Code Opus with help from Claude Code Fable and
JJ Behrens.

## Inspiration

My first programming language was Borland's Turbo Pascal running in DOS or
Windows 3.1. I've always thought that Pascal was the Python of its day in that
it was a really pleasant language to code in and was great for newbies. I had
all these Turbo Pascal programs that I had written as a student, but I had no
way to run them. I'm really inspired by the idea of creating a native
compiler, but there's hardly a point now that LLVM exists. I also wanted a
chance to play with Rust a bit more and to use Claude. Hence, my project to
transpile Turbo Pascal code to Rust. If you have some old Turbo Pascal code
lying around, give it a try!

## Usage

You need a Rust toolchain (`rustup`, stable). From the repository root:

```sh
# Transpile, build and run a program. The Cargo project goes in out/<name>/.
cargo run -p pascrust -- run examples/life.pas

# Arguments after `--` go to the Pascal program (ParamStr / ParamCount).
cargo run -p pascrust -- run myprog.pas -- first second

# Just print the generated Rust.
cargo run -p pascrust -- emit examples/paper.pas

# Transpile and build, without running. `-o` picks the project directory.
cargo run -p pascrust -- build examples/magic_square.pas -o /tmp/magic
```

Old sources with DOS (`\r\n`) line endings are fine everywhere: the
transpiler, `Readln` on standard input and text files, and `fmt` (which
writes UNIX line endings).

Or install the tools once with `cargo install --path crates/pascrust`, which
puts `pascrust` and `tpc` (below) on your `PATH`, and use
`pascrust run foo.pas`.

### Formatting Pascal sources: `pascrust fmt`

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
cargo run -p pascrust -- fmt examples/database/*.pas

# Only check: list the files that would change, and fail if there are any
# (handy in CI; this repository's tests check examples/ this way).
cargo run -p pascrust -- fmt --check examples/*.pas examples/*/*.pas
```

### A Turbo Pascal style compiler: `tpc`

`tpc` takes the command line of Turbo Pascal's `TPC.EXE`, so old build
scripts and habits carry over. `tpc game` compiles `game.pas` (the extension
is optional, and names match ignoring case, as in DOS) and writes the
executable `game` next to it, as `TPC GAME` wrote `GAME.EXE`. The Cargo
project is built in a cache directory (`~/.cache/pascrust/tpc/`), so only the
program appears beside the source.

```sh
cargo install --path crates/pascrust      # once; or use target/debug/tpc

tpc life                                  # life.pas -> ./life
tpc -Ebin '-Uunits;lib' game              # executable in bin/, units also in units/ and lib/
tpc -Q '-$I-' backup                      # quiet, and I/O errors go to IOResult
```

| Option | Meaning |
| --- | --- |
| `-$<switches>` | Compiler switches, as at the top of each file: `-$I-`, `-$R+,I-`. Quote them in a Unix shell, which would read `$I` as a variable. |
| `-E<dir>` | Where to write the executable. |
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
| `PASCRUST_TERM=plain` | Don't take over the terminal for `Crt`; `ReadKey` reads standard input. This is automatic when stdin or stdout isn't a terminal, so you can script a program: `printf 'y\nq' \| pascrust run game.pas`. |
| `PASCRUST_FAST=1` | Skip `Delay`. |
| `PASCRUST_GRAPH=headless` | Draw `Graph` output without opening a window (for tests; `ReadKey` then reads standard input). |
| `PASCRUST_GRAPH_SAVE=screen.bmp` | Save the graphics screen to a BMP file at `CloseGraph` or `Halt`. |
| `PASCRUST_DRIVE_C=/some/dir` | Where the DOS drive `C:` lives for file names like `C:\DATA\X.DAT` (default: the directory the program started in). Any drive letter works, names match files on disk ignoring case, `A:X` is relative to the current directory when `ChDir` went somewhere on drive A, and `..` stops at the drive's root, as in DOS. |
| `PASCRUST_PRINTER=...` | Where the `Printer` unit's `Lst` prints: a file name (default `printer.txt`), or `\|command` to pipe to a command such as `\|lp`. See [Printing](#printing). |

### Printing

Programs that `uses Printer` print with `Writeln(Lst, ...)`. There's no
printer port any more, so where the output goes is up to you, through the
`PASCRUST_PRINTER` environment variable:

```sh
# The default: append everything printed to printer.txt, in the directory
# the program started in.
pascrust run report.pas

# Append to a file of your choice.
PASCRUST_PRINTER=/tmp/report.txt pascrust run report.pas

# Send it to a real printer (macOS and Linux), or to any other command.
PASCRUST_PRINTER='|lp' pascrust run report.pas
PASCRUST_PRINTER='|lp -d office' pascrust run report.pas
PASCRUST_PRINTER='|less' pascrust run report.pas
```

The quotes matter, because `|` means a pipe to the shell. A command is run
by `sh`, starts the first time the program prints, and gets the output on
its standard input. When the program ends, its input is closed and PascRust
waits for it to finish, so the whole job is sent. A program that never
prints creates no file and runs no command. Output goes through exactly as
written, form feeds (`Write(Lst, #12)`) included. A procedure that writes
to a `var f: Text` parameter can be handed `Lst` (or `Output`) to print a
report instead of showing it. Old programs often wrote
printer control codes for an Epson or IBM printer, and those reach your
printer unchanged too.

### What's supported

Most of Turbo Pascal 5.5/6.0 as students used it: the usual types, strings,
arrays, records, enums, sets, pointers, nested procedures, `with`, units,
objects with inheritance and virtual methods (constructors, destructors,
`inherited`, polymorphic pointers, `TypeOf`), text and typed files with
`{$I-}`/`IOResult`, and the `Crt`, `Dos`, `Graph`, `Printer` and `Overlay`
units. Some things a modern machine doesn't have are faked so old programs
still run: `Lst` prints to a file or a command (see [Printing](#printing)),
overlays need no `.OVR` file, `MemAvail` reports plenty of memory.

Not supported yet: variant records, procedural types, `goto`, untyped files,
and `PChar` with the `Strings` unit. See `language_spec.md` for the details
and `TODO.md` for what's next. Runtime errors print Turbo Pascal's message
and exit code (e.g. `Runtime error 201: Range check error ...`).

## Examples

`examples/` holds the Turbo Pascal programs this project was built to run,
written in 1996-97.
Run them from the repository root:

```sh
# Text-mode programs (Crt)
cargo run -p pascrust -- run examples/blkjack.pas        # Blackjack
cargo run -p pascrust -- run examples/paper.pas          # Paper, scissors, rock
cargo run -p pascrust -- run examples/life.pas           # The Game of Life
cargo run -p pascrust -- run examples/magic_square.pas   # Solves a 4x4 magic square puzzle
cargo run -p pascrust -- run examples/gmscode.pas        # Encodes what you type as you type it
cargo run -p pascrust -- run examples/menu.pas           # A DOS menu (its choices ran other .exe files)

# Multi-file programs: run the main file, and its units come along
cargo run -p pascrust -- run examples/airport/airport4.pas   # An airport runway simulation

# A record filing system. Drive C: is mapped to its directory, so when it
# asks which file to open, type `mark` to load the sample data (mark.dat, mark.key).
PASCRUST_DRIVE_C=examples/database cargo run -p pascrust -- run examples/database/filesys.pas

# Graphics (Graph): these open a window
cargo run -p pascrust -- run examples/clock/tester.pas   # A clock whose hands run backwards
# Tic-tac-toe expects to live in A:\TICTAC, so map drive A: to examples/.
PASCRUST_DRIVE_A=examples cargo run -p pascrust -- run examples/tictac/tictac.pas
```

The environment variable goes before `cargo`, so it applies to the whole
command (and to the program it runs).

## Developing

See `CLAUDE.md` for the layout and commands, `architecture.md` for the design,
and run `cargo test` for the whole test suite.

## Acknowledgements

PascRust's code is its own, but we leaned on a lot of other people's work
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
  description of the language (see the References in `language_spec.md`).
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
