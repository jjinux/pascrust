# Calling Turbo Pascal from Rust

A Rust program can use a Turbo Pascal unit (and the units it uses). The
easy way is to let `cargo build` translate it, with a build script; the
other is to make a library crate of it once, with `pascrust build --lib`.
(The other way round, Turbo Pascal calling Rust, is in [Calling
Rust](calling_rust.md).)

## With a build script

`examples/pascrust/rust_calls_pascal` is a whole Rust project that uses
the user's `examples/database/bstunit.pas`, a binary search tree written
in 1997 (`cargo run --manifest-path examples/pascrust/rust_calls_pascal/Cargo.toml`).
It needs three things:

```toml
# Cargo.toml
[dependencies]
pascrust_runtime = { git = "https://github.com/jjinux/pascrust", rev = "..." }

[build-dependencies]
# The same `rev` as the runtime. Without the default features, it's just
# the translator, without the command-line tools.
pascrust = { git = "https://github.com/jjinux/pascrust", rev = "...", default-features = false }

# Turbo Pascal's integer arithmetic wraps around (32767 + 1 is -32768);
# Rust's stops with an error in a debug build unless this is off.
[profile.dev]
overflow-checks = false
```

```rust
// build.rs
fn main() {
    pascrust::build::library("pascal/bstunit.pas").unwrap();
}
```

```rust
// src/main.rs
pascrust_runtime::include_library!(bstunit);

use bstunit::{Bst, BstUnitContext, PeopleKey};
use pascrust_runtime::{RuntimeError, call};
```

- `pascrust::build::library` translates the unit into one file in Cargo's
  `OUT_DIR`, and tells Cargo to run it again when the unit, a unit it
  uses, or a Rust file it links (`{$L}`) changes. A Pascal error stops
  the build with PascRust's message; warnings show as Cargo's.
- `include_library!(bstunit)` is the module `bstunit`, named after the
  file (lowercased: `BSTUnit.pas` is `bstunit`). Lints are allowed in it,
  since nobody edits generated code.
- Turn on the runtime features the units need (`features = ["crt"]` for
  `Crt`, `["dos"]` for `Dos` or `WinDos`); if one is missing, the program
  doesn't compile, with a message saying which. A Rust file the unit
  links that uses crates (`//! cargo:` lines) needs them in
  `[dependencies]` too; the build says so.
- The Rust program needs edition 2018 or later. Two libraries that both
  use one unit each get their own copy of it.
- For hacking on PascRust itself, `path` dependencies on a checkout work
  too, as in the example.

## Overflow checks

A Pascal program's own Cargo.toml turns Rust's overflow checks off, so
its arithmetic wraps around as in Turbo Pascal. A library is compiled
with the Rust program's settings instead, and a debug build checks: then
`X + 1` at 32767 is a runtime error (255) instead of -32768. So:

- with a build script, the Pascal is part of the program's crate:
  `[profile.dev] overflow-checks = false` (all of the program);
- with `pascrust build --lib`, it's its own crate:
  `[profile.dev.package.bstunit] overflow-checks = false` (just the
  library).

If checks are on, the library's context says so on stderr the first time
it's made.

## With `pascrust build --lib`

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
is `Halt`'s exit code. With `call`s inside each other (Pascal calls Rust,
which `call`s Pascal), the innermost one gets it, and the Rust code
around it decides what happens next.

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

PascRust has no global state: a unit's variables are passed to its
routines as parameters. So for a library, the variables of the unit and
of the units it uses become the fields of one struct, named after the unit
(`CounterContext` for `Counter`), and the unit's interface routines are its
methods, with the fields passed in:

```rust
use counter::pascrust_runtime::{RuntimeError, call};
use counter::CounterContext;

fn main() -> Result<(), RuntimeError> {
    // `new` sets the variables up and runs the units' initialization
    // parts, which are Pascal code, so it goes through `call` too.
    let mut c = call(CounterContext::new)?;
    call(|| c.bump(5))?;
    println!("{}", c.current());
    Ok(())
}
```

- Every interface routine is a method (`&self` if it changes no unit
  variable), so a Pascal change in which variables a routine uses doesn't
  change the Rust API. A Pascal `New` is `new_2`: `new` makes the struct.
- The fields are public, named as in the generated code: when two units
  both have a `Count`, the second is `count_2` (each field's comment says
  which unit it's from). Typed constants are fields too.
- Every library has a context, even one whose units have no variables
  (`BstUnitContext {}` for `BSTUnit`): its `new()` runs the units'
  initialization parts, so a Rust program always starts with
  `call(XContext::new)`.
- The routines of the units it uses aren't methods (in Pascal, a program
  using `Counter` doesn't see `Helper`'s routines). Like the methods of an
  object type, or a procedural value, they take the unit variables they
  use as parameters: `tally.add(&mut c.count_2, 3)`. `pascrust emit`
  shows a routine's parameters.
- A method borrows the whole context, so `c.swap(&mut c.count_2, &mut x)`
  is a Rust borrow error; call the function instead,
  `counter::swap(&mut c.log, &mut c.count_2, &mut x)`.

`tests/embed/counter_host.rs` shows all of this, with `tests/embed/counter.pas`.
