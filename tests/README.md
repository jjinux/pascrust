# Tests

Three layers (see `architecture.md` §21):

1. **Unit tests** inside the crates (`cargo test -p pascrust --lib`): lexer
   token streams, parser shapes, sema type rules, runtime formatting.
2. **Translation tests** (`tests/translate/*.pas`): the generated Rust for
   each file is snapshotted with [insta](https://insta.rs). These document
   which Rust each Pascal construct becomes. Snapshots live in
   `crates/pascrust/tests/snapshots/`. `tests/errors/*.pas` are programs that
   must be rejected, and their snapshots are the error messages.
3. **Execution tests** (`tests/run/*.pas`): transpiled, compiled into one
   scratch Cargo package, and run. Standard output must match `NAME.out`
   byte for byte. `NAME.in` is fed to standard input if present, and
   `NAME.exitcode` holds the expected exit code (default 0). Every test
   needs a `.out` that isn't empty: a program that silently prints nothing,
   or can't print at all, must not pass.

Expected outputs come from Free Pascal in Turbo Pascal mode where possible
(`tests/gen_expected.sh`), so they are checked against a real Pascal compiler
rather than written from memory. Free Pascal and Turbo Pascal differ in a few
places (notably the default format of `Real` output, where FPC prints more
digits); those `.out` files are written by hand to match Turbo Pascal, and
say so in a comment at the top of the `.pas` file.

## Self-checking tests

Many execution tests check themselves: a `Check(ok, what)` procedure
prints `ok: what` or `FAILED: what` for each check, and the program ends
with `N of N checks passed` (and `Halt(1)` if any failed). The `.out`
lists every check by name, so a check that fails, is skipped, or never
runs, a program that stops early, and a program whose output is broken all
make the test fail, through the output, the exit code, or both. Copy the
`Check` procedure from `tests/run/system_queries.pas`.

Where Free Pascal agrees with Turbo Pascal, the `.out` comes from
`tests/gen_expected.sh`. Where it doesn't (`SizeOf`, DOS paths), the `.out`
is built from the `Check` calls in the source, never from pascrust's own
output, so it can't just repeat a pascrust bug.
