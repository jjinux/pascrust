# Pascrust

A Turbo Pascal → Rust transpiler, written in Rust. It's a learning project:
the user is an intermediate Rust programmer who's fascinated by language
implementation. The opening prompt is in `README.md`.

## Where things are decided

- `TODO.md`: the phased plan. Check items off as they land, and add new ones.
- `architecture.md`: core design decisions and their rationale. If you
  deviate from it, update it in the same commit (and say why).
- `language_spec.md`: which Pascal constructs are supported, by tier.
- `notes/`: research (p2c, Free Pascal) and answered questions for the user.

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
