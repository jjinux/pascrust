# Plan: a Rust project that builds its Pascal with `build.rs`

The user's question (2026-11-03): if a Rust project embeds Pascal code,
how does its `cargo build` know to run PascRust? Today a user runs
`pascrust build --lib unit.pas --out dir` by hand and depends on `dir` by
path (docs/embedding.md); a change in the Pascal needs that step again.

## Goal

```toml
# the Rust project's Cargo.toml
[dependencies]
pascrust_runtime = { git = "https://github.com/jjinux/pascrust", features = ["crt"] }

[build-dependencies]
pascrust = { git = "https://github.com/jjinux/pascrust", default-features = false }
```

```rust
// build.rs
fn main() {
    pascrust::build::library("pascal/bstunit.pas").unwrap();
}
```

```rust
// src/main.rs
pascrust_runtime::include_library!(bstunit);   // `mod bstunit { ... }`

use bstunit::{Bst, BstUnitContext, PeopleKey};
use pascrust_runtime::call;
```

`cargo build` then translates the unit (and the units it uses, and the
Rust files it links) whenever one of them changed, and only then.

## How

1. **`pascrust::build::library(path)`** (new module `crates/pascrust/src/build.rs`):
   - calls `driver::transpile_library` with the unit's directory and
     `$CARGO_MANIFEST_DIR`-relative paths;
   - writes **one file**, `$OUT_DIR/<crate name>.rs`, with every module
     inline (`pub mod helper { ... }`), since `include!` can't bring in
     files that declare `mod x;` (those would be looked for next to the
     including file) and an included file can't start with inner
     attributes or `//!` docs. The library's `#![allow(...)]` goes on the
     wrapper module the macro writes; the `//!` header becomes `//`;
   - prints `cargo:rerun-if-changed=` for every file it read: the unit,
     each unit it uses, each linked `.rs` (the driver returns the list:
     a new `Generated::sources`);
   - turns warnings into `cargo:warning=` lines and an error into a
     panic with the rendered message, which Cargo shows.
   - `library_with(path, &Options)` for unit directories and switches.
2. **Module paths**: the generated code names other units as
   `crate::helper` (codegen/mod.rs: unit imports, `use crate::Stats` for
   program types, `use crate::sound` for linked Rust, the dispatch enum
   path). Nested inside the user's crate those are wrong. For a library,
   codegen writes `super::helper` instead (units are siblings inside the
   library's module), behind a flag in `codegen::Input` (`library`
   already says it's a library). `pascrust build --lib` keeps working,
   since `super::` from a module at the crate root is `crate::`.
3. **`pascrust_runtime::include_library!(name)`**: a `macro_rules!` that
   expands to
   `#[allow(unused_variables, unused_assignments, dead_code, arithmetic_overflow)] pub mod name { include!(concat!(env!("OUT_DIR"), "/name.rs")); }`.
4. **Runtime features**: the generated code needs the runtime's `crt` or
   `dos` feature when the unit uses `Crt` or `Dos`; a build dependency
   can't turn on a dependency's features. So the generated file starts
   with a check that fails clearly at compile time:
   `const _: () = assert!(pascrust_runtime::FEATURE_CRT, "bstunit uses Crt: turn on pascrust_runtime's \"crt\" feature");`
   with `pub const FEATURE_CRT: bool = cfg!(feature = "crt");` (and
   `FEATURE_DOS`) in the runtime. The `build` helper also prints a
   `cargo:warning` naming the features.
5. **Keeping the build dependency light**: `pascrust`'s only dependency
   is `clap`, for the binaries. Put it behind a default `cli` feature
   (`required-features = ["cli"]` on the two `[[bin]]`s), so
   `default-features = false` builds just the library.
6. **Versions**: the transpiler and the runtime must come from the same
   revision (the generated code uses runtime items by name). Docs say to
   use the same `git`/`rev` for both; a `RUNTIME_VERSION` const checked by
   the generated file (`assert!` on a version string) would catch a
   mismatch, maybe later.

## Tests

- `crates/pascrust/tests/build_rs_tests.rs`: writes a small Rust project
  into `CARGO_TARGET_TMPDIR` with path dependencies on this repo's two
  crates, a `build.rs` calling `pascrust::build::library`, and a
  `main.rs` like `tests/embed/counter_host.rs`; runs `cargo run`; checks
  the output. Then touches `helper.pas` and checks Cargo rebuilds
  (or at least that the output file names all sources).
- A unit test that the single-file output has no `crate::` and no inner
  attributes, and that the `rerun-if-changed` list has the unit, the units
  it uses, and a linked `.rs`.
- The `pascrust build --lib` tests stay as they are (`super::` paths).

## Docs

docs/embedding.md gets a "Building with `build.rs`" section (the
three files above), and the `pascrust build --lib` way stays as the
manual one; README's embedding line mentions it; docs/architecture.md
§22 explains the single file and `super::`.

## Open questions

1. `include_library!` in the runtime, or have the user write the `mod`
   with `include!` themselves (one less macro, but they must get the
   `#[allow]` right)?
2. Is a git dependency the right way to depend on PascRust until it's on
   crates.io, or should the docs show a path dependency on a checkout?

## After Fable's review (2026-11-04), and the user's answers

Fable prototyped the plan end to end (a host whose `build.rs` translates
four libraries into one file each in `OUT_DIR`, `-D warnings`, editions
2021 and 2024) and recommended these changes, now the plan:

1. **One file with inline modules**, as planned. Only the included file's
   own top can't have `//!` or `#![...]`; inside `mod x { ... }` the units'
   headers stay as codegen writes them. (`mod x;` with files in `OUT_DIR`
   resolves inconsistently; don't.)
2. **`super::` only in unit modules** (`imports()` and `type_path`), behind
   the library flag; `lib.rs` never has `crate::`.
3. **`include_library!`** allows `unused_imports`, `unused_variables`,
   `unused_assignments`, `dead_code`, `arithmetic_overflow`, `missing_docs`
   and `clippy::all` (the user: the user can't edit generated code). It
   lives in the runtime; `env!("OUT_DIR")` expands in the user's crate.
4. **Overflow checks** (a bug of `--lib` today too: a library is compiled
   with the host's profile, so `X + 1` at 32767 panics in a debug build).
   Checks are set per crate at compile time, so there's no switching them
   off in `call`. The user's choice: the docs give the `Cargo.toml` lines
   (`[profile.dev.package.<lib>] overflow-checks = false` for `--lib`;
   `[profile.dev] overflow-checks = false` for `build.rs`, where the
   Pascal is in the program's own crate), and the runtime probes once,
   in `XContext::new()`/`call`, and warns on stderr if checks are on.
5. **`rerun-if-changed`** needs `Generated::sources`: every file the driver
   opened (the unit, each used unit's path as `find_file` found it, each
   linked `.rs`), absolute. Built-in units need none; no directories.
6. **Errors**: `pascrust::build::library` returns `Result<(), build::Error>`
   whose `Debug` writes the rendered message as is (so `.unwrap()` and
   `fn main() -> Result<..>` show the carets); warnings become one
   `cargo:warning=` per line.
7. **Runtime features**: the `const _: () = assert!(pascrust_runtime::FEATURE_CRT, ...)`
   check works and is reported first; no `cargo:warning` for it.
8. **`cli` feature**: works as planned (`required-features` on both bins).
9. **`//! cargo:` crates of a linked `.rs`**: a `cargo:warning` naming the
   crates to add to the program's `[dependencies]` (the user's choice).
10. **Docs**: git dependencies with the same `rev` for both crates (the
    user's choice), a path dependency for hacking on PascRust; edition
    2018+; the macro's name is the file stem lowercased; the overflow
    lines; two libraries using one unit get two copies of it.

A full crate written into `OUT_DIR` can't work (Cargo reads dependencies'
manifests before build scripts run), and a separate `pascrust-build`
crate gains nothing over a `pascrust::build` module.
