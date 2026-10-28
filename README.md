Pascrust
========

Pascrust transpiles Turbo Pascal programs into Rust. It writes a small Cargo
project for each program, so you can build and run your old Pascal code
natively, and read the Rust it turned into.

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
file:

```sh
cargo run -p pascrust -- run examples/airport/airport4.pas
cargo run -p pascrust -- run examples/database/filesys.pas
```

The standard units `Crt`, `Dos` and `Graph` are built into the runtime.
`Graph` programs open a 640x480 window, and keys typed into it go to
`ReadKey`. The tic-tac-toe game expects to live in `A:\TICTAC`, so map drive
A: to the examples directory:

```sh
cargo run -p pascrust -- run examples/clock/tester.pas
PASCRUST_DRIVE_A=examples cargo run -p pascrust -- run examples/tictac/tictac.pas
```

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

## Developing

See `CLAUDE.md` for the layout and commands, `architecture.md` for the design,
and run `cargo test` for the whole test suite.

## Opening prompt

For the sake of posterity, here's the opening prompt:

I am building a transpiler to transpile Pascal to Rust.

I am a software engineer building a sample project to practice using Claude and Rust. I'm very fascinated by implementing programming languages, but I've never implemented a compiler for a language like Pascal.

I know a bit of Rust, but I'm not an expert. Code it the way that an intermediate Rust programmer would. If you use interesting language constructs, please leave a comment that explains the
useful programming construct. If there's anything interesting about how Pascal works, leave useful comments for that too.

Before writing any code, I want you to start by creating a plan using Fable. Then, Opus can take over and do most of the coding. Save the plan to a file named TODO.md. Start with the following TODOs:

- [x] Figure out a name:
  - [x] Pascrust
- [ ] Write a TODOs.md file.
- [ ] Have Fable fill out the TODOs.md with the plan and make the core architectural decisions.
- [ ] Opus can then read the plan and fill in the details.
- [ ] Write a CLAUDE.md.
- [ ] Use an MIT license.
- [ ] Figure out if I should aim to translate Pascal or Turbo Pascal:
  - [ ] Is there an official spec and/or open source imlementation for Turbo Pascal?
  - [ ] Wasn't there some famous Pascal to C compiler? If so, carefully study how it's implemented. Take notes and save them into a file.
  - [ ] Write a file called language_spec.md with a plan of what language constructs we want to support.
- [ ] Make the core architectural decisions:
  - [ ] Figure out what lexer and parser I should use.
  - [ ] Maybe we should have a "virtual heap" that represents the Pascal heap in a somewhat safe way. Definitely our code can't have the same memory safety guarantees that Rust has. The code doesn't need to be fast. It just needs to run safely and correctly inside Rust. Passing around an object that represents the Pascal heap is an idea. The idea of embedding a Pascal engine inside Rust is not unreasonable. Maybe using a memory-mapped file as the Pascal heap makes sense to keep the heap separate. Errors can cause the engine to do something like raising an exception to the Rust container.
  - [ ] How should we represent arrays and strings in Pascal in Rust?
  - [ ] How should we represent pointers and the heap?
  - [ ] How should we represent records?
  - [ ] How do we build a FFI to Rust?
  - [ ] How do we build a FFI to C?
  - [ ] Is it even possible to have a FFI to Pascal that's already been compiled? This is low priority, so maybe we can just save some ideas for later. Initially, we can assume all of the Pascal code will be compiled to Rust.
  - [ ] Can we skip implementing threads? Do Pascal / Turbo Pascal even have those built in?
  - [ ] Write all of the core architectural decisions to a file named architecture.md.
- [ ] Start building a battery of tests to verify we're compiling it correctly:
  - [ ] Are there existing tests in other open source projects that we can use for inspiration?
  - [ ] There should be tests that test what Pascal constructs translate into what Rust constructs.
- [ ] How do we build at least a small standard library, including things like printing to stdout?
  - [ ] There should be tests that compile Pascal to Rust and then run the Rust to generate expected output.
- [ ] Is there an official standard library for Pascal or Turbo Pascal that has an interface that we can match?
- [ ] If possible, copy all the code in @~/Dropbox/programming/turbo_pascal to an examples directory and get them working.

Take your time. Ask me questions. I am happy to provide guidance.

Feel free to add new TODOs in TODO.md, and please mark ones that you've already done as done.

The name of this project is "Pascrust". That's a combination of "Pascal and Rust", but also points to Pascal acting like a crust on top of Rust, or Rust being a crust with embedded Pascal running in the center.

Commit your work to a new Git repo as you go along.