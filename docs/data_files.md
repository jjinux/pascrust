# Data files as JSON

A Turbo Pascal program's typed file (`file of T`) is its records' bytes,
one after another, in TP's layout. `pascrust to-json` turns one into JSON,
so old data can be read, searched or moved to something else, and
`pascrust from-json` turns the JSON (edited, say) back into the file the
program reads:

```sh
cargo run -p pascrust -- to-json examples/database/founit.pas examples/database/mark.dat
```

```json
[
  {
    "name": "Shannon Behrens",
    "ID": 1,
    "Index": 2930802.0
  }
]
```

`mark.dat` was written by the user's database program in 1997. Nothing in
the file says what its records look like, so the first argument is the
program or unit that declares the type (the units it uses are found as for
`pascrust build`).

- `--type T` says which type the records have, in any case (`people`,
  `People`), or a file type (`MasterFileType`, a `file of People`).
- Without `--type`, it's the `T` of the only `file of T` in the source.
  `founit.pas` has two (`People` and `PeopleKey`); then it's the one whose
  size fits the file's length (`mark.key`, 14 bytes, is one `PeopleKey`),
  and if both fit, it says so and asks for `--type`.
- `-o file.json` writes the JSON to a file; without it, to the screen.

A file whose length isn't a whole number of records is an error: it's
probably the wrong type.

## The JSON

An array, one element per record. Each value is in the JSON form of its
type:

| Pascal | In the file (TP 7) | JSON |
|---|---|---|
| `ShortInt`, `Byte`, `Integer`, `Word`, `LongInt` | 1, 1, 2, 2, 4 bytes, low byte first | a number |
| `Boolean` | 1 byte | `true` (any byte but 0, as in TP) or `false` |
| `Char` | 1 byte, code page 437 | a one-character string (`"é"` for #130) |
| `string[n]` | a length byte, then `n` bytes | the string; the bytes after its length are leftovers TP never cleared, and are dropped |
| `Real` | 6 bytes | a number, written as the shortest decimal that gives the same 6 bytes (`0.1`, not `0.10000000000002274`) |
| an enumerated type | 1 byte (2 for more than 256 values) | the value's name as the source spells it (`"Blue"`) |
| `set of T` | a bit per possible element | an array of the elements (`["Red", "Blue"]`, `["a", "z"]`) |
| `array[lo..hi] of T` | the elements | an array, in index order (an array of arrays for `[1..2, 1..3]`) |
| a record | its fields, in order | an object, its keys spelled as in the source |
| an `object` without virtual methods | its parent's fields, then its own | an object |
| a pointer, `Pointer`, `PChar`, a procedural value, `Text`, `file` | 4, 4, 4, 4, 256, 128 bytes | `null`: an address or a file handle means nothing outside the run that wrote it |

A `file of Integer` is an array of numbers (`[1, 2, 3]`), and so on.

A variant record (`case Kind: Shape of ...`) has its fixed fields and its
tag, then only the fields of the variant the tag picks. Without a tag
(`case Integer of ...`) nothing says which variant was meant, so every
variant's fields are there, each read from the same bytes:

```json
{ "W": 258, "Lo": 2, "Hi": 1 }
```

## Back again: `from-json`

```sh
cargo run -p pascrust -- to-json examples/database/founit.pas examples/database/mark.dat -o mark.json
# ...edit mark.json...
cargo run -p pascrust -- from-json examples/database/founit.pas mark.json -o mark.dat
```

It takes the same source and `--type`, and the JSON as `to-json` writes it.
It's strict, since a wrong byte would corrupt the program's data; each of
these is an error that says where (`[3].Where.X`):

- a field missing, or a key that isn't a field (field names in any case);
- a number that doesn't fit its type (300 for a `Byte`), or isn't whole;
  a real too big or too small (but not 0) for a `Real`;
- a string longer than its `string[n]`, or a character that isn't in code
  page 437, which TP's text used;
- an enumeration name that isn't one of the type's values, or a set
  element that can't be in the set;
- an array with the wrong number of elements;
- a pointer (or `Text`, `file`, ...) that isn't `null`;
- in a variant record with a tag, a field of another variant than the one
  the tag picks.

In a variant record without a tag, the fields of any variants may be there,
as `to-json` writes them all: each is written where it lives, and it's an
error only if two put different values in the same byte (`{"W": 258,
"Lo": 3}`, where `Lo` is `W`'s low byte, 2). A Boolean `true` agrees with
any byte but 0, since that's how TP reads it.

The bytes after a string's length, and those of a variant that's shorter
than the longest, are written as zeros. So `to-json` then `from-json` gives
back the file, except for leftovers TP never cleared (in `mark.dat`, 13
bytes after the name), and `from-json` then `to-json` gives back the JSON.

## What it can't read

- `Single`, `Double` and `Extended`: TP stores them in 4, 8 and 10 bytes,
  but PascRust (in translated programs too) stores every real as a 6-byte
  `Real` for now, so both commands refuse them rather than use the wrong
  bytes.
- Subranges (`Month: 1..12`) are checked against their host type only
  (`Byte`), so `from-json` accepts 13.
- An `object` with virtual methods: in TP its bytes start with a pointer
  to the type's method table, which PascRust doesn't model.
- An enumeration value past the type's last one: an error naming the
  record and field, since it has no name.
