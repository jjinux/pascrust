Pascrust
========

[![CI](https://github.com/jjinux/pascrust/actions/workflows/ci.yml/badge.svg)](https://github.com/jjinux/pascrust/actions/workflows/ci.yml)

Pascrust is a Turbo Pascal to Rust transpiler, so that you can more easily
run your old Turbo Pascal programs on modern computers. It writes a small
Cargo project for each program, so you can build and run your old Pascal code
natively, and read the Rust it turned into.

Pascrust was written by Claude Code Opus with help from Claude Code Fable and
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

# Tidy up indentation, spacing and identifier casing. Pass a program and its
# units together so names are spelled the same in all of them.
cargo run -p pascrust -- fmt examples/database/*.pas
```

Old sources with DOS (`\r\n`) line endings are fine everywhere: the
transpiler, `Readln` on standard input and text files, and `fmt` (which
writes UNIX line endings).

Or install the tool once with `cargo install --path crates/pascrust` and use
`pascrust run foo.pas`.

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
| `PASCRUST_DRIVE_C=/some/dir` | Where the DOS drive `C:` lives for file names like `C:\DATA\X.DAT` (default: the directory the program started in). Any drive letter works, and names match files on disk ignoring case, as in DOS. |

### What's supported

Most of Turbo Pascal 5.5/6.0 as students used it: the usual types, strings,
arrays, records, enums, pointers, nested procedures, units, objects (without
virtual methods yet), text and typed files, and the `Crt`, `Dos` and `Graph`
units. See `language_spec.md` for the details and `TODO.md` for what's next.
Runtime errors print Turbo Pascal's message and exit code
(e.g. `Runtime error 201: Range check error ...`).

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
