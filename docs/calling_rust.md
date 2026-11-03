# Calling Rust from Turbo Pascal

A Turbo Pascal program can call routines written in Rust, the way TP
programs called routines written in assembly: declare them `external`,
and say which file has them with `{$L ...}`. TP linked an `.OBJ` file;
PascRust links a `.rs` file.

```pascal
program Beeper;
{$L sound.rs}

procedure Beep(Hz: Word); external;
function Twice(N: Integer): Integer; external;

begin
   Beep(440);
   WriteLn(Twice(21));
end.
```

```rust
// sound.rs, next to beeper.pas
use pascrust_runtime::prelude::*;

pub fn beep(hz: u16) {
    pwriteln!("beep at ", hz, " Hz");
}

pub fn twice(n: i16) -> i16 {
    n * 2
}
```

`pascrust run beeper.pas` prints `beep at 440 Hz` and `42`.

## How it works

- `{$L sound.rs}` names a Rust file, next to the program (or in a unit
  directory). It becomes a module of the generated crate, `sound`, as it
  is. A program or unit can link several.
- Each `external` routine is the Rust function of the same name in snake
  case (`DoubleIt` is `double_it`), in the file of the last `{$L}`
  before it. It must be `pub`.
- The arguments are in PascRust's usual types (below). The generated
  code says what each function must look like:

  ```rust
  // Pascal: `procedure Beep(Hz: Word);`
  // Written in Rust, in `sound.rs`: `pub fn beep(hz: u16)`
  ```

  so `pascrust emit beeper.pas` is the quickest way to see the signature
  to write.
- A unit's routines can be in Rust too: put the `{$L}` and the
  `external` declarations in its `implementation` part.
- An `external` routine can't be nested in another routine, and doesn't
  see the program's global variables: pass what it needs.

## Types

| Pascal | Rust |
|---|---|
| `ShortInt`, `Byte`, `Integer`, `Word`, `LongInt` | `i8`, `u8`, `i16`, `u16`, `i32` |
| `Real`, `Single`, `Double`, `Extended` | `f64` |
| `Boolean` | `bool` |
| `Char` | `u8` (a byte, as in TP) |
| `string`, `string[n]` | `PString` (`pascrust_runtime::PString`) |
| an enum, record or object of the program | its Rust type, `crate::TPoint` |
| `array[1..10] of T` | `PArray<T, 1, 10>` |
| `^T` | `Ptr<T>` |
| a `var` parameter | `&mut T` |
| a function's result | the return type |

`PString` has `as_bytes()` and `to_string()` to read it, and
`PString::from(...)` (from a `&str` or bytes) to make one. Write output
with the runtime's `pwrite!`/`pwriteln!`, as the generated code does, so
it goes where Pascal's `Write` goes (the `Crt` screen, a redirected
file); `println!` works too, but bypasses that. To stop the program with
a Turbo Pascal runtime error, call `run_error(code)`.

## Crates

A Rust file can use crates from crates.io. List each one on a line of its
own at the top, as you would in `Cargo.toml`, after `//! cargo:`:

```rust
//! cargo: rand = "0.8"

use rand::Rng;

pub fn roll() -> i16 {
    rand::thread_rng().gen_range(1..=6)
}
```

The line goes into the generated project's `Cargo.toml`, so `pascrust
run` fetches the crate as Cargo does.

## In Turbo Pascal

TP 7 can't link a `.rs` file, so a program that calls Rust compiles only
with PascRust. PascRust can't link `.OBJ` files either (they're 16-bit
x86 code for DOS); `{$L sound.obj}` is an error that says to write the
routines in Rust.
