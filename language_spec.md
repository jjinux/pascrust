# PascRust language specification

## Decision: Turbo Pascal, not ISO Pascal

PascRust translates **Turbo Pascal** (Borland's dialect), accepting Turbo
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
  change semantics (`{$B+}`, `{$R+}`, `{$Q+}`); `{$M}`, `{$N+}` and the
  overlay ones (`{$O+}`, `{$O UnitName}`, `{$F+}`) are ignored silently;
  include files (`{$I name}`) are an error
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
- Constant expressions in `const` sections, array bounds and `case` labels,
  with TP 7's constant functions: `Abs`, `Chr`, `Hi`, `High`, `Length`,
  `Lo`, `Low`, `Odd`, `Ord`, `Pred`, `Round`, `SizeOf`, `Succ`, `Swap`,
  `Trunc`. A constant that is an enum value (`const Last = Sun`) has the
  enum's type, and `Succ`/`Pred` of one must stay in the enum. A call
  with constant arguments (`Hi(-1)`) is written as its value
- Function calls, including calling a function name as the result variable
  inside its own body
- Operator precedence exactly as TP (`not` > `* / div mod and shl shr` > `+ -
  or xor` > relational)

Statements

- Assignment; compound `begin ... end`; `if`/`then`/`else` (dangling else
  binds to the nearest `if`)
- `while`, `repeat ... until`, `for ... to/downto` (ordinal loop variables of
  any ordinal type), `case ... of` with lists and ranges (of numbers,
  chars, enums and Booleans) and `else`. As in TP, a label may repeat or
  overlap an earlier one, and the first wins; a label the value can never
  equal (300 for a `Byte`, a range like `5..1`) is an error
- Procedure calls; `Exit`; `Halt` / `Halt(n)`
- Value and `var` parameters; passing arrays, records and strings both ways.
  Different fields of one record may be passed as two `var` arguments
  (`Init(p.x, p.y)`, also through a pointer: `q^.x, q^.y`,
  `arr[i]^.x, arr[i]^.y`, `q^.a[1], q^.y`); the same place
  twice, a record and its field, a pointer and a field behind it
  (`F(q, q^.x)`), or two elements of one array may not

Built-ins (System unit)

- `Write`, `Writeln`, `Read`, `Readln` on the console, with `:width` and
  `:width:decimals` formatting, TP's default real format, `TRUE`/`FALSE`
- `Ord`, `Chr`, `Succ`, `Pred`, `Inc`, `Dec`, `Odd`, `Abs`, `Sqr`, `Sqrt`,
  `Sin`, `Cos`, `ArcTan`, `Ln`, `Exp`, `Trunc`, `Round`, `Int`, `Frac`, `Pi`,
  `Random`, `Random(n)`, `Randomize` (the same sequence as TP; see
  References), `MaxInt`, `MaxLongInt`
- `SizeOf` (TP's sizes: `Real` is 6 bytes, `string[n]` n+1, records
  unpadded), `High`/`Low` (arrays, strings, ordinal types), all three
  constants; `Swap` (TP 7 documents "X is an expression of type Integer
  or Word; result type same as parameter". As TP's compiler and Free
  Pascal do, a Byte swaps as a Word and a ShortInt as an Integer; a
  constant folds to the Word made of its low 16 bits swapped, so
  `Swap(255)` is 65280 and `Swap(-1)` 65535; a LongInt trades its 16-bit
  halves, as Free Pascal allows), `Assigned`, `RunError`
- `FillChar` and `Move` on any variables, as TP's bytes: between
  different types, part of a record or array, with any value. A count past
  the end of a variable is runtime error 201 (TP overwrote whatever came
  next), and an error at compile time when it's a constant
- Untyped files (`file`): `Reset(f, RecSize)`/`Rewrite(f, RecSize)` (128
  bytes if not given), `BlockRead(f, Buf, Count [, Result])` and
  `BlockWrite` into and from any variable's bytes (also from inside one:
  `BlockWrite(f, s[1], Length(s))`), `Seek`/`FilePos`/`FileSize`/`Eof`/
  `Truncate` in records. Without `Result`, reading fewer than `Count`
  records is runtime error 100; with it, only the bytes read change
- The untyped `Pointer`: it holds any typed pointer (`p := node`), is cast
  back with `PNode(p)^.x` (a cast to the wrong type is runtime error 204),
  compares with typed pointers and `nil`, and gets raw memory from
  `GetMem(p, Size)`, which `p^` passes as untyped memory (`BlockRead(f,
  p^, n)`, `Move(p^, x, n)`). `GetMem` for a typed pointer allocates the
  whole type, which covers the dynamic-array idiom (`GetMem(a, n *
  SizeOf(Integer))` with `a: ^array[0..999] of Integer`). `FreeMem(p,
  Size)` frees either
- `PChar` and the `Strings` unit (TP 7's extended syntax, `{$X+}`, on as in
  TP 7): string literals and zero-based `array[0..N] of Char` where a
  `PChar` is wanted; `P^`, `P[i]` (read and written); `P + I`, `I + P`,
  `P - I`, `P - Q` (a `Word`), `Inc(P)`/`Dec(P, N)`; `=`/`<>`/`<`/`>` and
  `nil`; `Write(P)`; `GetMem`/`FreeMem`, `Assigned`, and casts to and from
  `Pointer`. All 21 routines of `Strings`: `StrLen`, `StrEnd`, `StrMove`,
  `StrCopy`, `StrECopy`, `StrLCopy`, `StrPCopy`, `StrCat`, `StrLCat`,
  `StrComp`, `StrIComp`, `StrLComp`, `StrLIComp`, `StrScan`, `StrRScan`,
  `StrPos`, `StrUpper`, `StrLower`, `StrPas`, `StrNew`, `StrDispose`.
  Reading or writing past the end of a string's memory is runtime error
  201 (TP did no check). A `Char` array used as a `PChar` is lent a copy
  of its characters for the statement (see architecture.md), so a `PChar`
  into an array can't outlive the statement: `P := A`, `P := StrScan(A,
  '.')`, returning one, or passing the array to a routine that keeps its
  `PChar` parameter is an error at compile time (a routine that only
  reads or writes the characters, or keeps them in a local, is fine).
  Not yet: `Read`, `Str`, `Val`, `Assign` and `Rename` on `Char` arrays
  and `PChar`s, the `WinDos` unit
- Untyped `var` parameters (`procedure Dump(var Data; Size: Word)`): the
  routine sees the caller's variable as TP's bytes, through a typecast
  (`TBytes(Data)[i]`, read or written; `TRec(Data)` read, or assigned as a
  whole), and can pass it on. An argument can start inside a variable
  (`Dump(s[1], 3)`, `Dump(buf[10], 5)`). The bytes are copied in and
  written back when the routine returns; nothing else can see the
  variable in between, so a program can't tell
- System variables (and the units' `TextAttr`, `DosError`, ...; these can be
  read and set with `:=`, but not passed as `var` arguments or to `Inc`):
  `ExitCode` (the exit code when the program ends),
  `RandSeed`, `FileMode` (how `Reset` opens typed files), and `Input` and
  `Output` as file arguments (`Writeln(Output, x)`), also passed to a
  `var f: Text` parameter (`Report(Output)`); they're always the console,
  so `Assign`/`Reset`/`Rewrite` on them is an error
- `GetDir`. Drive C: is the file system's root (`/`) unless
  `PASCRUST_DRIVE_C` says otherwise, so `GetDir` and `FExpand` give the
  real path (`C:\USERS\JJ\GAMES`); other drives exist only when mapped
  (`PASCRUST_DRIVE_A`), and a name on an unmapped drive isn't found.
  File names with a drive letter go to that drive's directory; `A:X` is relative to the current directory when
  it's on drive A (PascRust keeps one current directory, not one per
  drive), and `..` stops at the drive's root. `GetDir` and `FExpand`
  agree: after `ChDir('A:\SUB')`, `GetDir(1)` is `A:\SUB` and
  `FExpand('A:X')` is `A:\SUB\X`. `Assign(f, '')` is standard input and
  output, as in TP
- Faked, so old programs run: `MemAvail`/`MaxAvail` (512,000 bytes),
  `Mark`/`Release` (records are freed when nothing points to them, so they
  do nothing), `SetTextBuf` (files are buffered already), `Test8087` (3)
- `Length`, `Copy`, `Pos`, `Delete`, `Insert`, `Concat`, `UpCase`, `Str`,
  `Val` (as Free Pascal: a number too big for the variable wraps with code
  0, a minus sign is an error for a Byte or Word, and after an error the
  variable is 0)
- `div` and `mod` of the most negative Integer by -1: runtime error 200,
  as TP's divide trap gave
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
  passing a descendant where an ancestor is expected (by `var`, or by value
  and in assignments, which copy the ancestor part: unlike TP, the copy
  then acts as the ancestor, since TP also copied the virtual method table)
- `private` keyword accepted and ignored
- `virtual` methods: dispatch through `Self`, through `var` parameters of an
  ancestor type, and through pointers (a pointer to an object may point to a
  descendant); `constructor`/`destructor` (`New(p, Init)` must name a
  constructor and `Dispose(p, Done)` a destructor), `inherited Method`,
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
  `Seek`, `FilePos`, `FileSize`, `Eof`, `Erase`, `Truncate`; records are
  stored in Turbo Pascal's own layout, the size `SizeOf` gives: a
  `string[n]` takes n+1 bytes, a `Real` TP's 6-byte format, a set of an
  enum or Boolean as few bytes as its values need, so files written by TP
  programs (`examples/database/mark.dat`) can be read
- File variables inside records and objects; `var` file parameters
- `Input` / `Output` as explicit file arguments

More language

- Sets: `set of T` (`T` with ordinals 0 to 255: `Char`, `Byte`, `Boolean`,
  an enumeration, a subrange like `0..9`), literals with ranges
  (`['a'..'z', '_']`), `in`, `+ - *`, `= <> <= >=`, `Include`/`Exclude`,
  set constants and typed constants, sets in records (also in `file of`
  records, as 32 bytes) and parameters. A value outside the element range
  is never in a set (`300 in s` is False for a `set of Byte`, for a
  constant or a variable; Free Pascal cuts a variable down to a byte)
- `with` statements (single and multiple record expressions, records and
  objects, through pointers). A body may not change an array index that
  chooses the record (`with a[i] do ... i := 2` is an error), and a field
  can't be a `for` loop's variable
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
  `TextBackground`, `Window`, `WindMin`, `WindMax`, `HighVideo`,
  `LowVideo`, `NormVideo`, `TextMode`, `ReadKey`, `KeyPressed`,
  `Delay`, `Sound`, `NoSound`, `InsLine`, `DelLine`, `AssignCrt` (a `Text`
  file on the screen and keyboard), colour constants `Black..White`,
  `Blink`, mode constants (`BW80`, `CO80`, ...)
- Variables: `TextAttr` (read and assign; it holds both colours),
  `LastMode` (set by `TextMode`), `CheckBreak` (when False, Ctrl-C comes
  back from `ReadKey` as `#3` instead of ending the program); faked, so
  they're remembered but change nothing: `DirectVideo`, `CheckSnow`,
  `CheckEOF`
- A non-TTY "plain" mode so scripted tests can drive these programs

Printer unit

- `Lst`, a text file always open on the printer: `Write(Lst, ...)`,
  `Writeln(Lst, ...)`, `Flush(Lst)`, and passed to a `var f: Text`
  parameter (`Report(Lst)`). Faked: it appends to `printer.txt`,
  another file, or a command such as `lp`, as `PASCRUST_PRINTER` says (see
  the README). Reading from `Lst`, or reopening it, is an error

Overlay unit (faked: a Rust program has all its code in memory)

- `OvrInit`, `OvrInitEMS` (both succeed; no `.OVR` file is needed),
  `OvrSetBuf`/`OvrGetBuf`, `OvrSetRetry`/`OvrGetRetry` (remembered),
  `OvrClearBuf` (does nothing); `OvrResult` (read and assign),
  `OvrFileMode`, `OvrLoadCount` and `OvrTrapCount` (always 0); the
  `ovrOk`...`ovrNoEMSMemory` constants. `{$O+}`, `{$O UnitName}` and
  `{$F+}` are ignored silently

Dos unit

- `GetDate`, `GetTime`, `FExpand`, `FSearch`, `FSplit` (parts cut to
  `DirStr`/`NameStr`/`ExtStr`; a shorter string variable is an error, as
  TP wants those types), `PathStr`/`DirStr`/`NameStr`/`ExtStr`,
  `Exec`, `DosExitCode`, `DosError` (a variable), `GetEnv`, `EnvCount`,
  `EnvStr`; `ChDir`/`MkDir`/`RmDir`/`GetDir` are in System
- `FindFirst`/`FindNext` with `SearchRec` and the attribute constants
  (`Directory`, `AnyFile`, ...): DOS wildcards (`*.*` matches every name),
  directories only with `Directory`, dot-files count as `Hidden`, matches
  in name order, `DosError` 18 when there are no more and 3 for a missing
  directory. `?` also matches nothing at the end of a name (`DATA?.DAT`
  finds `DATA.DAT`), `.` and `..` are listed in every directory but a
  root (`\`, or a drive's `A:\` wherever it's mapped), a link to a directory counts as a directory, and `VolumeID` alone
  finds nothing (there's no volume label). A copy of a `SearchRec` goes on
  from the same place by itself. `SearchRec.Name` is a full string (TP's
  was `string[12]`), so long names fit. Nothing tells PascRust when a
  program is done with a search, so it keeps up to 10,000 (of which up
  to 100 that have run out), forgetting those used longest ago
- Faked: `DosVersion` (DOS 6.22), `DiskFree`/`DiskSize` (512 MB free of a
  1 GB disk on every drive: real disks are too big for a `LongInt`),
  `SetDate`/`SetTime` (don't change the computer's clock),
  `GetCBreak`/`SetCBreak` and `GetVerify`/`SetVerify` (remembered),
  `SwapVectors` (does nothing)
- Refused with a clear error, since they need real DOS: `Intr`, `MsDos`,
  `Keep`, `GetIntVec`, `SetIntVec` (`Registers` is declared, with its
  `Word` registers, so a program using it gets that error)
- `GetFTime`/`SetFTime` (the file's real modification time, packed as DOS
  did), `PackTime`/`UnpackTime` with `DateTime` (seconds kept to 2; as in
  TP, an out-of-range field spills into the next, within the date or
  the time), and
  `GetFAttr`/`SetFAttr` on a `Text` or typed file: `ReadOnly` and
  `Directory` are real; `Hidden` is a name starting with `.`; `Archive`,
  `SysFile` and a `Hidden` set by `SetFAttr` are remembered while the
  program runs (faked: modern file systems have nowhere to keep them).
  They follow the file through `Rename`, and writing to it sets `Archive`
  again, as in DOS. `SetFTime` on a file open for writing sticks after
  `Close`, as in DOS, and works on a read-only file (except on Windows,
  which wants write access); a time that doesn't exist locally (the hour skipped
  when clocks go forward, day 0) moves to the nearest one that does.
  `DosError` is 2 for a missing file and 3 for a missing directory.
  TP's untyped `var F` took any variable; PascRust wants a file
- `Erase` of a read-only file fails with access denied (5), as in DOS
- DOS path translation (`a:\tictac` → a configured directory)

Graph unit (BGI, as used in the examples)

- `InitGraph`/`Detect`/`CloseGraph`/`GraphResult`/`GraphErrorMsg`/
  `SetGraphMode`/`RestoreCrtMode`, `GetMaxX`/`GetMaxY`/`GetMaxColor`
- `SetColor`/`GetColor`/`SetBkColor`/`GetBkColor`, the 16 colour constants
- `PutPixel`/`GetPixel`, `Line`/`LineTo`/`LineRel`/`MoveTo`/`MoveRel`/
  `GetX`/`GetY`, `Rectangle`, `Bar`, `Bar3D`, `Circle`, `Arc`, `Ellipse`,
  `PieSlice`, `Sector`, `FillEllipse`, `FloodFill`, `GetArcCoords`
  (`ArcCoordsType`)
- `DrawPoly`/`FillPoly` with an array of `PointType` (TP's untyped `var
  PolyPoints` took any variable; PascRust wants that array). `DrawPoly`
  doesn't close the shape; `FillPoly` fills it (even-odd rule) and
  outlines it closed. A `NumPoints` beyond the array's length uses the
  array's points (TP would read past it). Out-of-range `SetPalette`,
  `SetAllPalette` and `SetFillStyle` arguments change nothing, as the
  BGI's `grError` did
- `SetLineStyle`/`GetLineSettings` (`SolidLn`, `DottedLn`, ..., `NormWidth`,
  `ThickWidth`, `LineSettingsType`), `SetFillStyle`/`SetFillPattern`/
  `GetFillPattern`/`GetFillSettings` (`FillPatternType`,
  `FillSettingsType`), `SetWriteMode`
- `OutText`/`OutTextXY`, `SetTextJustify` (`LeftText`, `CenterText`,
  `RightText`, `TopText`, `BottomText`), `SetTextStyle` (default font only;
  scaled sizes), `GetTextSettings` (`TextSettingsType`),
  `TextWidth`/`TextHeight`
- `SetViewPort`/`ClearViewPort`/`ClearDevice`/`GetViewSettings`
  (`ViewPortType`), `GetAspectRatio`
- `SetPalette`, `GetPalette`/`GetDefaultPalette`/`SetAllPalette`
  (`PaletteType`; -1 leaves a colour), `GetPaletteSize`, `SetRGBPalette`
  (the VGA's 64 colour registers the EGA palette points into),
  `GraphDefaults`, `SetAspectRatio` (remembered; pixels stay square),
  `SetUserCharSize` (with `UserCharSize`), `GetDriverName` (`EGAVGA`),
  `GetModeName`/`GetMaxMode`/`GetModeRange`
- Faked: `InstallUserFont` gives a new font number (drawn in the built-in
  font), `SetGraphBufSize` does nothing. Refused: `RegisterBGIdriver`,
  `RegisterBGIfont` and `InstallUserDriver`, which link or load driver
  code; PascRust's one driver is built in, so the calls can go
- `SetActivePage`/`SetVisualPage`: two pages in every mode (VGA had two
  only at 640x350), for flicker-free animation
- `GetImage`/`PutImage`/`ImageSize`, with `NormalPut`/`CopyPut`,
  `XORPut`, `OrPut`, `AndPut` and `NotPut`. The buffer (`p^` after
  `GetMem(p, ImageSize(...))`, or any variable) holds the image in TP's
  EGA/VGA layout: two `Word`s (width - 1, height - 1), then each row as
  bit planes 3 down to 0, 8 pixels a byte (`ImageSize` counts 2 bytes
  more, which `GetImage` leaves alone). So sprites saved by
  the original programs still draw. `ImageSize` of 64K or more is 0 with
  `GraphResult` `grError`. A buffer too small is runtime error 201.
  `PutImage` is clipped to the viewport (with `ClipOn`)

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
6. (Untyped `var` parameters, `GetMem`/`FreeMem` and the untyped `Pointer`
   are done, in tier 2.) Pointer arithmetic, `Addr`/`@` on variables (would need
   the place-descriptor representation from `architecture.md` §15)
7. (Untyped files are done, in tier 2.) `TextRec`/`FileRec`
8. (Done, now in tier 2: TP-exact binary layout for `file of Rec`.)
9. `const` parameters, open array parameters, `Include`/`Exclude` (TP 7
   additions). (`PChar` and `Assigned` are done, in tier 2.) A `PChar` kept
   into a `Char` array (`P := A`), and `WinDos`
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
