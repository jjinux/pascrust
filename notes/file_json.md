# Plan: `pascrust json` and `pascrust unjson` (a `file of T` to JSON and back)

The user's request (2026-10-29, prioritized 2026-11-05): turn a binary file
written by a Turbo Pascal program (`file of T`) into JSON. The user's
choices (2026-11-05):

- PascRust decodes the bytes itself, from the Pascal source's types (no
  generated program, nothing to compile).
- Both ways: JSON back to a `file of T`, so old data can be edited.
- Without a type name, the only `file of T` the source declares.
- A variant record shows only the variant its tag picks; without a tag,
  every variant.

## Commands

```sh
pascrust json   examples/database/founit.pas mark.dat --type People            # JSON on stdout
pascrust json   examples/database/founit.pas mark.dat --type People -o mark.json
pascrust unjson examples/database/founit.pas mark.json --type People -o mark.dat
```

- The first argument is the program or unit that declares the type. Its
  `uses` are found as for `pascrust build` (next to it, PascRust's own
  units), so a type from a used unit works too.
- `--type T` names a type the source can see at its top level (a unit's
  interface or implementation, a program's declarations). Without it: the
  `T` of the only `file of T` declared anywhere in the source (types,
  variables, fields: `founit.pas` has two, `People` and `PeopleKey`, so it
  says so and lists them).
- `json` writes to stdout unless `-o`; `unjson` needs `-o` (binary on a
  terminal helps no one).

## The JSON

An array with one element per record, pretty-printed, two spaces:

```json
[
  {
    "name": "Shannon Behrens",
    "ID": 1234,
    "Index": 1.5
  }
]
```

| Pascal | Bytes (TP 7) | JSON |
|---|---|---|
| `ShortInt`, `Byte`, `Integer`, `Word`, `LongInt` | 1, 1, 2, 2, 4, little-endian | number |
| `Boolean` | 1 (0 or 1) | `false`/`true`; another byte is an error in `json` (it's not a Boolean file, or the type is wrong) |
| `Char` | 1, CP437 | a one-character string |
| `string[n]` | length byte, then `n` bytes | the string of `length` characters, CP437 decoded; the bytes after it are junk TP never cleared, and are dropped |
| `Real` | 6, Real48 | number |
| enum | 1 (2 if over 256 values) | the value's name as declared (`"Green"`) |
| `set of T` | TP's bitmap bytes (`set_bytes`) | an array of the elements, as their own JSON form |
| `array[lo..hi] of T` | the elements | an array (an array of arrays for `[1..3, 1..4]`) |
| record | its fields in order, no padding | an object, keys as the fields are spelled in the source |
| variant record with a tag | fixed fields, the tag, the variant area | the fixed fields, the tag, the active variant's fields only (a tag with no variant: none) |
| variant record without a tag | the same, without the tag | every variant's fields, each decoded from the same bytes |
| pointer, `Pointer`, `PChar`, procedural | 4 | `null` (an address means nothing in another run); `unjson` writes nil |

Keys keep the source's spelling (`"ID"`, `"Index"`), since the JSON is for
people who know the Pascal. Field names are unique within a record in
Pascal, variant fields included, so an object never repeats a key.

## Not supported (an error that says why)

- `Text` and `file` fields (a file variable inside a record holds an
  OS handle: meaningless on disk).
- `object` types (they can't be in a typed file in TP if they have
  virtual methods, and a plain one is rare; can come later).
- `Single`, `Double`, `Extended`, `Comp`: PascRust maps them all to `Real`
  (6 bytes on disk), so a file written by the real TP with a `Double`
  field (8 bytes) doesn't read correctly, neither by a translated program
  nor here. **Open question 1.**
- A file whose size isn't a whole number of records: an error naming the
  record size and the leftover bytes.

## `unjson`

The inverse, strict, because a wrong byte corrupts the Pascal program's
data:

- every field must be there, and nothing else (error with a path,
  `[3].address.zip`);
- each number must fit its type, a string its `string[n]`, a character
  CP437; an enum name must be one of the type's values;
- a tagged variant record: the tag's value picks the variant, and only
  its fields may appear;
- a tagless variant record: the fields of exactly one variant may appear
  (open question 2);
- unused bytes (after a string's length, the rest of a variant area) are
  written as zeros.

So `json` then `unjson` gives back the same bytes except for the junk after
strings and in unused variant space; `unjson` then `json` gives back the
same JSON.

## Code

- `crates/pascrust/src/json/` (behind the `cli` feature, as it's a
  command): `layout.rs` turns a sema `Type` into a `Layout` tree (sizes and
  offsets, field names as spelled, enum names, set bounds), reusing
  `tp_size`/`set_bytes`/`RecordType::variant_area` so it agrees with what
  translated programs write; `decode.rs` (bytes → JSON value) and
  `encode.rs` (JSON value → bytes).
- `driver`: a new entry that runs the front end on the source (as `emit`
  does) and returns the `Analysis` and the type, without codegen.
- `serde_json` (with `preserve_order`, so keys stay in field order) as an
  optional dependency under `cli`, for parsing and printing. (A hand-written
  JSON parser is possible but isn't what this project is about.)
- Real48 and CP437: the runtime already has both (`io::files::real48`,
  `cp437`). Depend on `pascrust_runtime` (no features, so no extra crates)
  from `pascrust` rather than copying them, so the two can't disagree.

## Tests

- `crates/pascrust/tests/json_tests.rs`:
  - `examples/database/mark.dat` (a real file from 1997) with `founit.pas`
    and `People`, and `mark.key` with `PeopleKey`: snapshots of the JSON.
  - A Pascal test program (`tests/json/records.pas`) with every supported
    kind of field (integers, Boolean, Char, `string[n]`, Real, enum, sets
    of chars and enums, arrays of arrays, a nested record, tagged and
    tagless variants, a pointer) writes `records.dat` through `pascrust
    run`; `json` on it is a snapshot, `unjson` of that gives the same bytes
    (the program clears strings first with `FillChar`, so no junk), and a
    second Pascal program reads the `unjson` output back and prints it.
  - Errors: the wrong type size, two `file of`s without `--type`, a bad
    Boolean byte, each `unjson` error (snapshots of the messages).
- Free Pascal isn't an oracle here: its `Real` is 8 bytes, so its files
  differ from TP's.

## Docs

- README: the command, with the `mark.dat` example.
- `docs/data_files.md`: the table above, `unjson`'s rules, the limits.
- `docs/architecture.md`: a section on the layout code and why it reuses
  sema's sizes.

## Open questions

1. `Single`/`Double`/`Extended`/`Comp` are 4/8/10/8 bytes in TP's files
   but 6 in PascRust's (all are `Real`). Fix that first (a `Real` kind in
   sema; the runtime writes each in its TP size), or leave it as a known
   limit for both translated programs and this command?
2. A tagless variant record in `unjson`: fields from more than one
   variant (they overlap in the bytes): an error, or the later one wins?
3. Command names: `json`/`unjson`, or `dump`/`load`, or `to-json`/`from-json`?

## After Fable's review (2026-11-05)

Fable checked the plan against the code and the runtime; the byte table
matches what translated programs read and write (and TP 7). Changes to the
plan:

1. **Enum names as spelled**: `EnumType` keeps only Rust names
   (`camel_case`); add `names: Vec<String>` with the Pascal spellings.
2. **`--type` lookup**: case-insensitive, through `Analysis.symbols`
   (`SymKind::TypeName`); `founit.pas` declares `people` in lowercase. If it
   names a file type (`MasterFileType`), use its element type.
3. **A unit as the source**: the driver wraps it as `transpile_unit` does
   (`program Library; uses X;`) and stops after `sema::analyze`.
4. **Reals print as the shortest decimal that gives the same 6 bytes**
   (Real48 has 39 bits of mantissa, so `0.1` would otherwise print as
   `0.10000000000002274`).
5. **Boolean**: any nonzero byte is `true`, as in TP and the runtime (a
   1990s record from stack junk may have other values). An enum byte past
   the last value is an error naming the record, field and offset.
6. **`Single`/`Double`/`Extended`/`Comp`** are all `Type::Real` today, so
   the decoder can't even tell them apart to refuse them. Open question 1.
7. **Tagless variants**: `json` prints every variant, so `unjson` must
   accept fields of several variants: open question 2.
8. **Nested variant parts**: the layout recurses (`VariantPart::offset_of`
   in a tagless part; in a tagged part, the active variant's fields, then
   its own nested tag and active variant).
9. **Objects without virtual methods** are supported like records (parent
   fields first, as `file_record!` does); with virtual methods, an error
   (TP has a hidden VMT word PascRust doesn't model).
10. **`Text`/`file` fields**: `null`, written back as zeros, like pointers.
11. **Subranges** are checked only against their host type in `unjson`
    (the bounds aren't in `Analysis`); a known limit.
12. **`unjson` and CP437**: a character outside CP437 is an error, not `?`.
13. **Any element type**: `file of Integer` is `[1, 2, 3]`; arrays indexed
    by Char or an enum are JSON arrays in index order.
14. **Default type**: among several `file of` types, the one whose size
    divides the file's length, if only one does (`mark.key`, 14 bytes, is
    `PeopleKey`); otherwise the error lists them.
15. **Dependencies**: `cli = ["dep:clap", "dep:serde_json", "dep:pascrust_runtime"]`,
    `serde_json` with `preserve_order`; `#[cfg(feature = "cli")] pub mod json;`;
    `json_tests` with `required-features = ["cli"]`. A build script's
    `default-features = false` pulls in nothing new.
16. **Tests**: binary fixtures checked in under `tests/json/` (one written
    by `fpc -Mtp`, without reals, so an independent compiler vouches for the
    layout), insta snapshots of `json` and of each error, `unjson` giving the
    same bytes, a unit test that each layout's size is `tp_size`, and a run
    test reading the fixture through a translated program.
17. A pre-existing mismatch: `tp_size` gives an enum of more than 256
    values 2 bytes, the runtime writes 1. The layout follows TP (2); the
    runtime gets a TODO.

Expected JSON for the 1997 files (hand-decoded):
`mark.dat` is `[{"name": "Shannon Behrens", "ID": 1, "Index": 2930802.0}]`
and `mark.key` is `[{"ID": 1, "Location": 0, "Before": null, "After": null}]`.

Fable's recommendations for the open questions: (1) keep the kind in the
type now (`Type::Real(RealKind)`, everything else unchanged), so `json`
refuses a `Double` field clearly, and put the full on-disk fix (4/8/10/8
bytes; about a day) on TODO; (2) `unjson` accepts fields of several
tagless variants, and errors only where two of them write different
values to the same byte; (3) `json`/`unjson`.

The user's answers (2026-11-05): (1) refuse now, fix later, as Fable
recommends; (2) an error only on a clash; (3) the commands are
`pascrust to-json` and `pascrust from-json`.
