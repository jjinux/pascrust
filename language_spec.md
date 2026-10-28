# Pascrust language specification

## Decision: Turbo Pascal, not ISO Pascal

Pascrust translates **Turbo Pascal** (Borland's dialect), accepting Turbo
Pascal 7.0 syntax and implementing the Turbo Pascal 5.5 feature level first.
ISO 7185 "Standard Pascal" is not a target.

Why:

- Every program in `examples/` is Turbo Pascal: `uses Crt`, `string[30]`,
  units, `object` types, `file of Rec`, `Inc`/`Dec`, `ReadKey`, `GotoXY`.
  ISO Pascal has no units, no strings of that kind, no objects, and a
  different file model; an ISO translator could not run a single example.
- Turbo Pascal is a strict superset of what matters in ISO 7185 for ordinary
  programs, so ISO-style code mostly works anyway. The ISO features TP lacks
  (conformant arrays, `pack`/`unpack`, `get`/`put` buffer variables, some
  `goto` forms) are rare in real code.
- TP 7.0 is the final and best-documented version. Its Language Guide is a
  complete, freely available specification (see `notes/p2c.md` §4). The
  examples predate TP 7 (they are 5.5-era), so we treat 5.5's feature set as
  the must-have and 7.0's additions as nice-to-have.
- Free Pascal's `-Mtp` mode is an open-source reimplementation of this exact
  dialect and gives us a reference compiler for test outputs.

Semantics follow TP 7.0 defaults unless noted: `{$B-}` short-circuit boolean
evaluation, `{$R-}` no range checks on subranges (we always check array
indexes anyway), `{$I+}` I/O errors abort, `{$N-}` software reals (`Real` is
mapped to `f64`), 16-bit `Integer`.

What "supported" means: sema accepts it, codegen produces Rust that compiles
and behaves like TP, and there is a test for it.

## Tier 1: MVP (hello world, then everything in a textbook chapter 1-8)

Program structure

- `program Name;` (parameters after the name are ignored, as in TP)
- `const`, `type`, `var` sections, repeated and in any order; `label` is
  parsed but `goto` is rejected (tier 3)
- `procedure` / `function` declarations, nested to any depth, recursive
- `forward` declarations
- Compiler directives `{$...}`: `{$I-}`/`{$I+}` switch I/O checking (see
  Files). The others are parsed and ignored, with a warning for ones that
  change semantics (`{$B+}`, `{$R+}`, `{$Q+}`); `{$M}` and `{$N+}` are
  ignored silently; include files (`{$I name}`) are an error
- Comments `{ }` and `(* *)`, including the examples' `{t ... }` headers
- Main block `begin ... end.`

Types

- `Integer`, `LongInt`, `ShortInt`, `Word`, `Byte`, `Real`, `Boolean`, `Char`
- `String`, `String[n]`
- Enumerated types `(Red, Green, Blue)`; subranges `1..10`, `'A'..'Z'`,
  `Red..Blue`
- `array[lo..hi] of T`, multi-dimensional `array[a..b, c..d]`, nested arrays,
  any ordinal index type (integer, char, enum, subrange); `packed` accepted and
  ignored
- `record ... end`, nested records
- Named types and type aliases

Expressions

- Integer and real arithmetic with TP's promotion rules; `/` is always real;
  `div`, `mod`; unary minus
- `and`, `or`, `not`, `xor` on booleans (short-circuit) and on integers;
  `shl`, `shr` (TP's: `shr` is a logical shift, and shifting by the type's
  width or more gives 0)
- Comparisons on numbers, chars, strings, booleans, enums
- String concatenation `+`, char/string mixing
- Integer, real (`1.5`, `1E3`), char (`'a'`, `#13`, `^M`), string (`'it''s'`),
  boolean literals; `nil`
- Typecasts between ordinal types `Integer(c)`, `Char(65)`, `Ord`, `Chr`
- Constant expressions in `const` sections and array bounds
- Function calls, including calling a function name as the result variable
  inside its own body
- Operator precedence exactly as TP (`not` > `* / div mod and shl shr` > `+ -
  or xor` > relational)

Statements

- Assignment; compound `begin ... end`; `if`/`then`/`else` (dangling else
  binds to the nearest `if`)
- `while`, `repeat ... until`, `for ... to/downto` (ordinal loop variables of
  any ordinal type), `case ... of` with lists and ranges and `else`
- Procedure calls; `Exit`; `Halt` / `Halt(n)`
- Value and `var` parameters; passing arrays, records and strings both ways.
  Different fields of one record may be passed as two `var` arguments
  (`Init(p.x, p.y)`, also through a pointer, `q^.x, q^.y`); the same place
  twice, a record and its field, or two elements of one array may not

Built-ins (System unit)

- `Write`, `Writeln`, `Read`, `Readln` on the console, with `:width` and
  `:width:decimals` formatting, TP's default real format, `TRUE`/`FALSE`
- `Ord`, `Chr`, `Succ`, `Pred`, `Inc`, `Dec`, `Odd`, `Abs`, `Sqr`, `Sqrt`,
  `Sin`, `Cos`, `ArcTan`, `Ln`, `Exp`, `Trunc`, `Round`, `Int`, `Frac`, `Pi`,
  `Random`, `Random(n)`, `Randomize` (the same sequence as TP; see
  References), `MaxInt`, `MaxLongInt`
- `SizeOf` (TP's sizes: `Real` is 6 bytes, `string[n]` n+1, records
  unpadded), `High`/`Low` (arrays, strings, ordinal types), all three
  constants; `Swap`, `Assigned`, `RunError`
- `Length`, `Copy`, `Pos`, `Delete`, `Insert`, `Concat`, `UpCase`, `Str`,
  `Val`
- `High`, `Low`, `SizeOf` (of types with a defined Rust size)

## Tier 2: needed to run every program in `examples/`

Modules

- `unit` with `interface` / `implementation` / initialization block; `uses`
  in programs and in both unit sections; name resolution across units
- Whole-program compilation from a program file that pulls in its units

Objects (TP 5.5 OOP)

- `object` types with fields and methods, implicit `Self` field access,
  method implementations `procedure T.M(...)`, nested procedures inside
  methods that use `Self` fields
- Single inheritance `object(Parent)`, inherited field and method access,
  passing a descendant where an ancestor is expected
- `private` keyword accepted and ignored
- `virtual` methods: dispatch through `Self`, through `var` parameters of an
  ancestor type, and through pointers (a pointer to an object may point to a
  descendant); `constructor`/`destructor`, `inherited Method`,
  `Ancestor.Method(...)`, `New(p, Init(...))`, `New(PType, Init(...))` as a
  function, `Dispose(p, Done)`, `TypeOf` (compared with `=`/`<>`). Not yet:
  a descendant hiding a static method that calls virtual ones

Pointers

- `^T` types, `New`, `Dispose`, `nil`, `p^`, `p^.field`, pointer equality,
  pointers inside records (linked lists, trees), `var` parameters of pointer
  type, passing `p^` and `p^.field` as `var` arguments

Files

- `Text` files: `Assign`, `Reset`, `Rewrite`, `Append`, `Close`, `Read`,
  `Readln`, `Write`, `Writeln`, `Eof`, `Eoln`, `SeekEof`, `SeekEoln`,
  `Erase`, `Rename`, `Flush`.
  `{$I-}` with `IOResult`: an I/O error (including a bad number in `Read`)
  sets `IOResult` instead of stopping the program, and further I/O is
  skipped until `IOResult` is read.
- `file of T`: `Assign`, `Reset`, `Rewrite`, `Close`, `Read`, `Write`,
  `Seek`, `FilePos`, `FileSize`, `Eof`, `Erase`, `Truncate`; records with
  `string[n]`, integer and real fields serialized to a fixed-size layout
- File variables inside records and objects; `var` file parameters
- `Input` / `Output` as explicit file arguments

More language

- Sets: `set of T` (`T` with ordinals 0 to 255: `Char`, `Byte`, `Boolean`,
  an enumeration, a subrange like `0..9`), literals with ranges
  (`['a'..'z', '_']`), `in`, `+ - *`, `= <> <= >=`, `Include`/`Exclude`,
  set constants and typed constants, sets in records and parameters
- `with` statements (single and multiple record expressions, records and
  objects, through pointers). A body may not change an array index that
  chooses the record (`with a[i] do ... i := 2` is an error)
- Typed constants (`const Days: array[0..6] of String[9] = (...)`;
  `const x: integer = 5`), which are initialized variables in TP
- Anonymous record types in `var` declarations (`EndPoint: record x, y:
  integer; end`)
- Char-indexed and enum-indexed arrays; `for` over chars and enums
- Shadowing of built-in names by user declarations (`procedure Exit(var
  leave: boolean)` and `type ShortString = string[20]` both appear in the
  examples)
- String indexing `s[i]`, `s[0]`; `Hi`, `Lo`, `Swap`; `FillChar` and `Move`
  on arrays of scalars (checked, no raw memory); `ParamCount`, `ParamStr`
- `Break`, `Continue` (TP 7; `Continue` in `repeat` still runs the `until`
  test)
- `{$N+}` types `Single`, `Double`, `Extended`, `Comp` as `f32`/`f64`/`f64`/`i64`

Crt unit (the whole examples set uses it)

- `ClrScr`, `ClrEol`, `GotoXY`, `WhereX`, `WhereY`, `TextColor`,
  `TextBackground`, `Window`, `WindMin`, `WindMax`, `TextAttr`, `HighVideo`,
  `LowVideo`, `NormVideo`, `TextMode`, `ReadKey`, `KeyPressed`,
  `Delay`, `Sound`, `NoSound`, `InsLine`, `DelLine`, `CheckBreak`,
  `DirectVideo` (ignored), colour constants `Black..White`, `Blink`
- A non-TTY "plain" mode so scripted tests can drive these programs

Dos unit (as used in the examples)

- `GetDate`, `GetTime`, `SetDate`/`SetTime` (no-ops with a warning),
  `FExpand`, `FSearch`, `FSplit`, `PathStr`/`DirStr`/`NameStr`/`ExtStr`,
  `Exec`, `DosExitCode`, `ChDir`/`MkDir`/`RmDir`/`GetDir` (System), `GetEnv`,
  `DosVersion`, `DiskFree`/`DiskSize` (best effort), `FindFirst`/`FindNext`
- DOS path translation (`a:\tictac` → a configured directory)

Graph unit (BGI, as used in the examples)

- `InitGraph`/`Detect`/`CloseGraph`/`GraphResult`/`GraphErrorMsg`/
  `SetGraphMode`/`RestoreCrtMode`, `GetMaxX`/`GetMaxY`/`GetMaxColor`
- `SetColor`/`GetColor`/`SetBkColor`/`GetBkColor`, the 16 colour constants
- `PutPixel`/`GetPixel`, `Line`/`LineTo`/`LineRel`/`MoveTo`/`MoveRel`/
  `GetX`/`GetY`, `Rectangle`, `Bar`, `Bar3D`, `Circle`, `Arc`, `Ellipse`,
  `PieSlice`, `DrawPoly`, `FillPoly`, `FloodFill`
- `SetLineStyle`/`GetLineSettings` (`SolidLn`, `DottedLn`, ..., `NormWidth`,
  `ThickWidth`), `SetFillStyle`/`SetFillPattern`/`GetFillPattern`/
  `GetFillSettings` (`FillPatternType`), `SetWriteMode`
- `OutText`/`OutTextXY`, `SetTextJustify` (`LeftText`, `CenterText`,
  `RightText`, `TopText`, `BottomText`), `SetTextStyle` (default font only;
  scaled sizes), `TextWidth`/`TextHeight`
- `SetViewPort`/`ClearViewPort`/`ClearDevice`/`GetViewSettings`,
  `GetAspectRatio`
- `GetImage`/`PutImage`/`ImageSize` on a byte buffer

## Tier 3: later, or never

Later (real TP features, not in the examples, worth doing in order)

1. (Done, now in tier 2: sets.)
2. Variant records (flattened; no type punning)
3. (Done, now in tier 2: `virtual` methods, constructors and destructors,
   polymorphic pointers to objects.)
4. Procedural types and procedure parameters (`type Func = function(x: real):
   real`), `@proc`
5. `goto` / `label` (only the structured uses: jump forward out of nested
   loops, which Rust can express with labeled `break`)
6. `GetMem`/`FreeMem`, untyped `Pointer`, `Addr`/`@` on variables (would need
   the place-descriptor representation from `architecture.md` §15)
7. Untyped `file`, `BlockRead`/`BlockWrite`; `TextRec`/`FileRec`
8. TP-exact binary layout for `file of Rec` (6-byte `Real`, exact record
   padding) to read the shipped `.dat`/`.key` files
9. `const` parameters, open array parameters, `string` as `PChar` with
   `{$X+}`, `Assigned`, `Include`/`Exclude` (TP 7 additions)
10. Vector (stroked) BGI fonts, `RegisterBGIfont`, custom drivers
11. `Overlay` unit: parse and ignore
12. `absolute` variables when both are scalars of the same size
13. The bundled-globals and place-descriptor options for pathological
    aliasing (see `architecture.md` §15)
14. Preserving Pascal comments in the generated Rust

Never (DOS/x86 machinery with no sensible Rust translation)

- `asm ... end`, `inline(...)`, `interrupt` procedures, `Intr`, `MsDos`,
  `GetIntVec`/`SetIntVec`, `Keep`
- `Mem`, `MemW`, `MemL`, `Port`, `PortW`, `Seg`, `Ofs`, `Ptr`, `CSeg`,
  `DSeg`, `SPtr`, `Addr` arithmetic, `absolute` at fixed addresses
- `Overlay` behaviour, `OvrInit` and friends (parse, ignore)
- `Exec` of real DOS executables (it will run the path through
  `std::process::Command`; `turbo.exe` will not be found)
- Direct video memory writes (`DirectVideo` is accepted and ignored)
- `far`, `near`, `export`, Windows (`TPW`) units
- ISO-only: conformant arrays, `pack`/`unpack`, `get`/`put` with file buffer
  variables `f^`, `program(input, output)` header parameters (ignored)

## References

Where we checked how Turbo Pascal actually behaves. When the documentation
and Free Pascal disagree, TP wins; `tests/gen_expected.sh` uses `fpc -Mtp`
for expected output only where FPC matches TP (it doesn't for `shr` or
16-bit `for` loops, whose expected output is written by hand).

- Borland's [*Turbo Pascal Version 7.0 Language Guide*](https://turbopascal.org/wp-content/uploads/Turbo_Pascal_Version_7.0_Language_Guide_1992.pdf)
  (1992): the de-facto spec, including the runtime error codes. For example,
  `Round` rounds halves away from zero. The TP 5.0 and 6.0 User's Guides are
  on [bitsavers](https://archive.decromancer.ca/bitsavers.org/pdf/borland/turbo_pascal/);
  more in `notes/p2c.md`.
- [daelsepara/turbo-pascal-assembly](https://github.com/daelsepara/turbo-pascal-assembly):
  annotated disassembly of TP's System unit. `docs/library/RANDOM-ENGINE.md`,
  `RANDOM-INT.md`, `RANDOM-REAL.md`, `RANDOM-REAL-FPU.md` and `RANDOMIZE.md`
  show exactly how `Random`, `Random(n)` and `Randomize` work, which our
  runtime follows (`crates/pascrust_runtime/src/system.rs`).
- [J R Stockton, "Borland Pascal/Delphi Random"](https://people.cs.nycu.edu.tw/~tsaiwn/sisc/runtime_error_200_div_by_0/www.merlyn.demon.co.uk/pas-rand.htm):
  the `RandSeed` update (`134775813 * seed + 1`), test values, and how
  `Random` differs between TP, BP 7 (`{$N+}`) and Delphi.
- [Delphi's Random Number Generator](https://groups.google.com/g/borland.public.delphi.language.delphi.general/c/TedBrYcVMYQ)
  (borland.public.delphi newsgroup): the same algorithm, unchanged since
  Turbo Pascal.
- Free Pascal's scanner (`compiler/scanner.pas`): where `^` is a pointer
  symbol and where `^M` is a character, which our lexer copies (see
  `caret_is_control_char` in `crates/pascrust/src/lexer.rs`).
- `fpc -Mtp` itself, for `{$I-}`/`IOResult`, `Eof` on a file being written
  (runtime error 104), `Real` formatting and most run tests.
