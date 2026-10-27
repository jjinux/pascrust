Pascrust
========

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