# Calling Turbo Pascal from Rust

`pascrust build --lib unit.pas` turns a Turbo Pascal unit (and the units
it uses) into a Rust library crate, so a Rust program can call it. (The
other way round, Turbo Pascal calling Rust, is in [Calling
Rust](calling_rust.md).)

Here the unit is `examples/database/bstunit.pas`, a binary search tree
written in 1997:

```sh
cargo run -p pascrust -- build --lib examples/database/bstunit.pas --out out/bstunit
```

`out/bstunit` is a Cargo project named after the file, `bstunit`. Its
`lib.rs` re-exports the unit's interface, so a Rust program depends on it
by path and uses it:

```toml
# Cargo.toml of the Rust program
[dependencies]
bstunit = { path = "../out/bstunit" }
```

```rust
use bstunit::pascrust_runtime::{RuntimeError, call};
use bstunit::{Bst, PeopleKey};

fn main() -> Result<(), RuntimeError> {
    let mut tree = Bst::default();
    call(|| tree.create())?;
    for (id, location) in [(50, 1), (30, 2), (70, 3), (60, 4)] {
        let mut item = PeopleKey { id, location, ..Default::default() };
        let mut ok = false;
        call(|| tree.insert(&mut item, &mut ok))?;
        println!("insert {id}: {ok}");
    }
    let mut item = PeopleKey { id: 60, ..Default::default() };
    let mut found = false;
    call(|| tree.search(&mut item, &mut found))?;
    println!("search 60: {found}, location {}", item.location);
    Ok(())
}
```

`tests/embed/bst_host.rs` is this program, complete, and
`crates/pascrust/tests/embed_tests.rs` builds and runs it.

## What the Rust sees

- The unit's types are Rust types with the usual names: the object `BST`
  is the struct `Bst`, its methods are methods (`tree.insert(...)`), and
  its records are structs with public fields that derive `Default`, so
  `PeopleKey { id: 60, ..Default::default() }` makes one.
- Parameters have PascRust's usual types (`Integer` is `i16`, a `var`
  parameter is `&mut`, a string is a `PString`): see the table in
  [Calling Rust](calling_rust.md#types). `pascrust emit` shows a unit's
  Rust.
- `bstunit::pascrust_runtime` is the runtime the library uses, with
  `PString`, `Ptr` and the rest, so the program needs no other dependency.

## Runtime errors

A Pascal runtime error (a range check, a nil or disposed pointer, a file
that isn't there) stops a Pascal program. Called from Rust through
`pascrust_runtime::call`, it comes back as an `Err(RuntimeError)` instead,
with Turbo Pascal's error number and message:

```rust
match call(|| other.search(&mut item, &mut found)) {
    Ok(()) => println!("found: {found}"),
    Err(error) => println!("{error}"),   // Runtime error 204: Invalid pointer operation: ...
}
```

Pascal code called without `call` still works, but a runtime error then
stops the Rust program (with Turbo Pascal's message). What the Pascal code
was changing when the error came may be left half done, as it would have
been in Turbo Pascal.

`Halt` in the Pascal code doesn't end the Rust program either: inside
`call` it comes back as an `Err` whose `halted` is true and whose `code`
is `Halt`'s exit code.

When the Rust program is done with the Pascal code, it can call
`pascrust_runtime::shutdown()`, which does what the end of a Pascal
program does: puts the terminal back if the unit used `Crt`, and finishes
`Lst` output to a printer command.

## What can't be a library

- A unit that uses `Graph` (itself or through another unit): `Graph`'s
  window needs the main thread, which only a Pascal program's runtime
  has. Make it a Pascal program that calls Rust instead, with
  [`{$L file.rs}`](calling_rust.md).
- A unit must be in a file named after it (`bstunit.pas` for `BSTUnit`),
  as in Turbo Pascal, since that's how `uses` finds units.

`tpc unit.pas` checks that a unit compiles, and says how to use it.

## Units with variables

A unit's global variables are passed to its routines as parameters
(PascRust has no global state), so the Rust program owns them and passes
them in. That's simplest when the unit keeps its state in one record, as
PascRust's own `PRTurtle` does (`Turtle: TTurtleContext`); or, like
`BSTUnit`, in an object the program creates. A unit with an
initialization part has a function for it (`pascrust emit` shows its
name), which the program calls first.
