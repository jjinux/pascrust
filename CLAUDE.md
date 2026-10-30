# PascRust

A Turbo Pascal → Rust transpiler, written in Rust. It's a learning project:
the user is an intermediate Rust programmer who's fascinated by language
implementation. The opening prompt is in `README.md` before it was removed
(`git show 726b781:README.md`).

## Where things are decided

- `TODO.md`: the phased plan. Check items off as they land, and add new ones.
- `docs/architecture.md`: core design decisions and their rationale. If you
  deviate from it, update it in the same commit (and say why).
- `docs/language_spec.md`: which Pascal constructs are supported, by tier.
- `notes/`: research (p2c, Free Pascal) and answered questions for the user.
- `README.md`: what a user needs: what PascRust is, how to run it (`pascrust`,
  `tpc`, `fmt`), the environment variables, what's supported, the examples.
- `docs/`: longer user documentation the README links to (printing, timer
  interrupts), so the README stays short. `docs/rust_limits.md` explains
  each error caused by a limit of Rust, with a worked rewrite; such an
  error links to its section (`sema::rewrite_link`, checked by
  `tests/docs_tests.rs`). A new error of that kind gets a section too.

Keep these up to date as you go: whenever a change adds or changes a
feature, a command, an option or a limitation, update `README.md`,
`docs/language_spec.md` and `docs/architecture.md` (and check off
`TODO.md`) in the same commit, not later. They drift quickly otherwise.

## Layout

- `crates/pascrust`: the transpiler (lexer → parser → sema → codegen → driver), bin + lib.
  `fmt/` is `pascrust fmt`, a source formatter that reuses the lexer.
- `crates/pascrust_runtime`: the library every generated program depends on
  (Pascal strings, I/O, `run()`, later `Crt`/`Dos`/`Graph` behind features).
- `tests/run/*.pas`: execution tests (`.out` expected stdout, optional `.in`, `.exitcode`).
- `tests/translate/*.pas`: snapshot tests of the generated Rust.
- `tests/errors/*.pas`: programs that must be rejected; snapshots of the messages.
- `examples/`: the user's own Turbo Pascal programs from the 90s, the acceptance test.

## Commands

```sh
cargo test                                   # everything
cargo test -p pascrust --test run_tests      # execution tests only
PASCRUST_TEST=hello cargo test --test run_tests   # one execution test
INSTA_UPDATE=always cargo test --test translate_tests   # accept new snapshots, then review the git diff
cargo run -p pascrust -- emit file.pas       # print generated Rust
cargo run -p pascrust -- run file.pas        # transpile, build and run (project goes in out/)
cargo run -p pascrust -- fmt examples/*.pas examples/*/*.pas   # keep the examples formatted (checked by fmt_tests)
tests/gen_expected.sh tests/run/foo.pas      # expected output from Free Pascal (fpc -Mtp)
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

## Licensing

PascRust is MIT. Don't copy code or tests from GPL projects (Free Pascal,
p2c, GNU Pascal): read them for ideas only. Running `fpc` as an external
program to produce expected output (`tests/gen_expected.sh`) is fine.

## Style rules

- Write Rust the way an intermediate Rust programmer would: clear over clever.
- When you use an interesting Rust construct (lifetimes, `RefCell`, trait
  tricks, const generics, `macro_rules!`, let-else, ...), leave a short
  comment explaining it. Same for interesting Pascal semantics.
- Generated code must also read like hand-written Rust: no redundant
  parentheses, casts or `mut`, and teaching comments where a Pascal construct
  needed an unusual translation.
- Generated code must compile without warnings.
- Every new Pascal construct needs a translate test *and* a run test.
- Commit as you go, with focused commits.

## Reviews

Have Claude Fable review all the work, the way you'd review every PR:
after each major feature, or every few commits, give a Fable subagent (the
Agent tool with `model: "fable"`) the commits since the last review (every
commit must be covered by one), with the project's goals, and ask it to
reproduce each finding with a small
Pascal program (compared with `fpc -Mtp` where FPC matches TP). Its scratch
files go in `out/review/`, and it must not read or search anything outside
the project. Fix the straightforward findings in follow-up commits (each
with a test that fails without the fix), and ask the user about the rest.
