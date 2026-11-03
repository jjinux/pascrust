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
  case (`DoubleIt` is `double_it`; a name that's a Rust keyword gets a
  trailing underscore, `Loop` is `loop_`), in the file of the last `{$L}`
  before it. It must be `pub`.
- A path in `{$L}` can use DOS's backslashes (`{$L rust\sound.rs}`).
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

## Rust calling back into Pascal

A Pascal routine can be passed to Rust as a procedural value, and the
Rust calls it: a sort that asks Pascal how to compare, an event loop
that calls Pascal handlers.

```pascal
program Callback;
{$L sorting.rs}
type
   TLess = function(A, B: Integer): Boolean;
   TList = array[1..6] of Integer;

procedure SortWith(var Items: TList; Less: TLess); external;

function Descending(A, B: Integer): Boolean;
begin
   Descending := A > B;
end;
...
   SortWith(Items, Descending);
```

```rust
// sorting.rs
use crate::{TLess, TList};

pub fn sort_with(items: &mut TList, less: TLess) {
    let less = less.get(); // the Pascal function, as a Rust `fn`
    for i in 2..=6 {
        let mut j = i;
        while j > 1 && less(items[j], items[j - 1]) {
            let t = items[j];
            items[j] = items[j - 1];
            items[j - 1] = t;
            j -= 1;
        }
    }
}
```

The program's types are in the crate root, so the Rust file names them
as `crate::TLess`. A procedural type is a `ProcVar`, and `.get()` gives
the routine as a plain Rust function pointer. A routine that uses the
program's global variables takes them as parameters too, so a callback
is simplest when it uses only its own parameters (`pascrust emit` shows
the type, `type TLess = ProcVar<fn(i16, i16) -> bool>;`).
`tests/run/rust_callback.pas` is this example, complete.

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
