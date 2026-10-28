# Prior art: p2c and friends

Research notes for Pascrust. The question from `TODO.md` was: "Wasn't there
some famous Pascal to C compiler? If so, carefully study how it's implemented."
Yes: **p2c**, by Dave Gillespie. This file records how it works, what we take
from it, and the other prior art (Free Pascal, test suites, Pascal to Rust
attempts, Turbo Pascal specs and standard-library interfaces).

## 1. p2c (Dave Gillespie, 1989-1993)

### What it is

- A Pascal to C translator written in C, about 30k lines, "version
  1.21alpha-07.Dec.93" is the last release by Gillespie (Synaptics). It is GPL
  (the *generated* code and the runtime `p2clib.c`/`p2c.h` are exempt).
  Announced on comp.compilers in 1989:
  https://compilers.iecc.com/comparch/article/89-11-022
- Sources (archived GitHub mirrors):
  https://github.com/FranklinChen/p2c and
  https://github.com/Classic-Tools/p2c. The `src/` directory holds the
  translator (`lex.c`, `parse.c`, `decl.c`, `expr.c`, `pexpr.c`, `funcs.c`,
  `trans.c`, `out.c`, `dir.c`, `comment.c`, `hpmods.c`, `citmods.c`,
  `stuff.c`, `trans.h`), the runtime (`p2clib.c`, `p2c.h`), the option file
  with every strategy documented (`sys.p2crc`), dialect interface files
  (`system.imp`, `turbo.imp`), the man page (`p2c.man`), and a `p2cc` Perl
  driver that makes p2c behave like a Pascal compiler.
- Input dialects: HP Pascal (default), Turbo Pascal ("TURBO (vers 5.0 for IBM
  PC)" per `sys.p2crc`), UCSD, Oregon, VAX, MPW, Modula-2 (partial). Turbo
  support is explicitly incomplete: `turbo.imp` has partial interface texts
  for the standard Turbo units and users "must write their own runtime
  emulations" for Crt/Dos/Graph. (That is exactly the gap Pascrust fills.)

### Pipeline (from `p2c.man`)

Read config (`sys.p2crc`, then `p2crc`, then command-line, then
`{Param=value}` comments embedded in the source) → parse the whole program
into an internal tree → optional flow analysis and dead-code removal → emit
`.c` (and a `.h` per unit/module with the interface translation). Comments are
"stamped with serial numbers" so they can be re-attached to the right
statement in the output, and long lines are broken with "the TeX typesetter's
paragraph formatter" algorithm. Stated goal: "Output code is arranged to be
readable and efficient." Pascrust adopts the whole-program approach, the
per-unit output file, comment preservation (tier 3) and, in spirit, the
readability goal; we delegate line breaking to `rustfmt`.

### How p2c handles the hard parts

**`var` parameters.** C has pointers, so `var x: T` is `T *x` and every use is
`*x`. Arrays are already pointers in C. Records passed by value rely on C
struct copying (`CopyStructs` option: "0=no implicit copying of structs
allowed ... 3=can assign, pass, and return structs; default=3"). A constant
passed to a `var` parameter gets a temporary (HISTORY, v1.20). File
parameters are special-cased: `VarFiles` "1 or default = `var f : file` params
use &; 0=pass FILE *'s by value; must not open or close the file within the
procedure".
*For Pascrust*: C does not have a borrow checker, so p2c never faces the
aliasing problem that `architecture.md` §15 is about. We keep the "`var` is a
reference" shape (`&mut T`) and add sema-level alias detection.

**Nested procedures.** p2c gathers the outer procedure's locals that inner
procedures use into a struct and passes a pointer to it. Gillespie, 1990, on
comp.lang.c
(https://www.tuhs.org/Usenet/comp.lang.c/1990-March/032197.html): the
translator collects "the parent's variables used by subprocedures into a
struct that the parent passes by pointer", and for deeper nesting "a
sub-procedure's struct includes a pointer to the parent's struct", so a
grandparent variable is reached as `LINK->LINK->v1`. Generated code looks like
`struct LOC_outer { int total; ... };` and `static void inner(int x, struct
LOC_outer *LINK) { LINK->total += x; }`. The `StaticLinks` option controls
whether procedure *pointers* carry a static link (`_PROCEDURE` = `{proc,
link}`): "1=use _PROCEDURE for proc ptrs; 0=use C function pointers, no
static links; default=1 in HP, 0 in Turbo Pascal". The man page admits "LINK
variables allowing sub-procedures access to their parents' variables are
occasionally omitted by mistake". Gillespie also notes that for a single
captured variable "passing a pointer to that variable directly would be
simpler" but p2c never did that.
*For Pascrust*: we do the "simpler" thing he describes, per-variable lifting,
because in Rust the struct-pointer scheme reintroduces the `&mut LOC` vs
`&mut LOC.field` double-borrow problem. See `architecture.md` §15.5.

**Sets.** Two representations: sets whose elements fit in `0..31` become "a
single integer variable using bit operations" (`SmallSetConst` controls
whether literals print as `1<<2|1<<4`, `0x14` or `20`); bigger sets are "an
array of long" where "element 0 counts the elements in use, and sets are
normalized so the last used element is nonzero". Runtime functions in
`p2c.h`: `P_setunion`, `P_setint`, `P_setdiff`, `P_setxor`, `P_inset`,
`P_setequal`, `P_subset`, `P_addset`, `P_addsetr` (ranges), `P_remset`,
`P_setcpy`, `P_expset`, `P_packset`; `SETBITS` is 32.
*For Pascrust*: one 256-bit `PSet<T>`; the small/large split is an
optimization we do not need.

**Strings.** Pascal strings become NUL-terminated C `char` arrays (so "some
length-byte tricks fail"), `string` without a size is `StringDefault` (255).
A `var s: string` parameter normally assumes `string[255]`; with
`VarStrings=1` a hidden size parameter is passed ("will match a string
variable of any size", readable through `strmax`). The Turbo setting "changes
the default integer size and string truncation limit", but "Turbo's automatic
string clipping is not supported". Runtime helpers: `strsub`, `strltrim`,
`strrtrim`, `strrpt`, `strpad`, `strpos2`, `sprintf` for `Str`.
*For Pascrust*: we keep the length-prefixed layout (`PString`, length byte
addressable as `s[0]`) and the per-variable capacity, so clipping works. The
hidden-size-parameter idea is why `PString` carries `cap` at run time.

**Files.** With `StructFiles=1` a file is a struct holding "the C FILE pointer
as well as file buffer and file name fields" (`_TEXT { FILE *f; int f_BFLAGS;
Char f_BUFFER; Char name[_FNSIZE]; }`); typed files use `FILEBUF(f, sc, type)`
macros with `GET`/`PUT`/`BUFEOF`/`BUFFPOS` emulating ISO buffer variables. I/O
status: `P_ioresult` and `P_escapecode` globals, `_SETIO(cond, ior)` sets
`P_ioresult` to 0 or the TP error code (`FileNotFound = 10`, `EndOfFile = 30`,
...). `CheckFileEOF` adds extra checks.
*For Pascrust*: `PText` and `PFile<T>` structs, the same TP `IOResult` codes,
and `{$I-}` state tracked in the runtime.

**`with` statements.** p2c introduces a temporary pointer to the record
(`WithName` option: "Name of WITH statement temp ptr; %s=unique id") and
rewrites field references through it.
*For Pascrust*: we rewrite to full paths instead of holding a temporary,
because a `&mut` temporary would lock the record for the whole block.

**Units / modules.** Each unit yields a `.h` with "a translation of the
module's interface section"; importing a unit means reading its Pascal
interface text, located via `ImportFrom` entries or `ImportDir` search
patterns (`%s.pas`, `%s.text`, `--HOMEDIR--/%s.imp`). `ImportAll` reads a
whole file "in case file contains several modules/units". Built-in units are
described by `.imp` files (`system.imp`, `turbo.imp`).
*For Pascrust*: one Rust module per unit, `pub` for interface items; built-in
units are Rust modules in `pascrust_runtime` whose Pascal-visible signatures
are a table in sema (the `.imp` idea, in Rust).

**Integers.** `Integer16`: "1=assume Pascal integers are 16 bits ... default=0
for HP/Oregon/VAX, 1 for Turbo/MPW". Pascrust: `i16` for Turbo, same
reasoning.

**Checks.** `ArrayCheck` "0 or default=do not check array bounds",
`NilCheck`, `CaseCheck` ("check CASE statements without OTHERWISE"),
`RangeCheck` "(not yet implemented)". Pascrust always checks array indexes
and nil derefs (Rust makes this free), and treats a `case` with no matching
arm and no `else` as a no-op (TP semantics).

**Known limitations listed by p2c** that we share or avoid: `goto` into/out of
blocks (p2c emulates non-local goto with `setjmp`; we reject), conformant
arrays, variant records (p2c uses C unions; we flatten), `computational` type.

### What Pascrust takes from p2c

1. Whole-program translation with a per-unit output file.
2. Lifting nested procedures to top level with explicit environment passing
   (we pass variables, p2c passes a struct pointer).
3. Length-prefixed strings with per-variable capacity, TP `IOResult` codes,
   TP integer widths.
4. The idea of a declarative table of built-in unit interfaces.
5. Comment preservation via source positions (tier 3).
6. The example corpus: p2c ships `examples/` (`basic.p`, `cref.p`, `e.p`,
   `fact.p`, `self.p`, ...) usable as extra tests once we accept HP-style
   `program` headers.

## 2. Other Pascal to Rust attempts

- **ferropascal** (https://github.com/sunsided/ferropascal, MIT, 2025-2026):
  "a modern Pascal-to-Rust transpiler". Uses a `pest` grammar, post-processes
  output with `clippy --fix` and `rustfmt`. Implemented: program structure,
  `var`/`const`, `writeln`, operators, `if`, `for`. Strings, records, arrays,
  procedures, file I/O are listed as *planned*. Takeaway: it is the only
  attempt we found, it stalled before any of the hard problems in
  `architecture.md`, and its pest-based parse-tree-then-AST design is a reason
  we chose a handwritten parser.
- **pascal-rs** (https://docs.rs/crate/pascal/0.1.2): a Pascal interpreter
  written in Rust, not a transpiler. Useful only as a reference for a Rust
  Pascal lexer/parser shape.
- Nothing else of substance exists; Pascrust is effectively green-field for
  the Rust-specific decisions (var aliasing, heap handles, lifting).

## 3. Free Pascal (the open-source Turbo Pascal)

- **Free Pascal** (https://www.freepascal.org) in `-Mtp` / `{$MODE TP}` mode is
  the de-facto open-source implementation of Turbo Pascal 7, including the
  `Crt`, `Dos` and `Graph` units with the same procedure names. Install on
  macOS with `brew install fpc` (https://formulae.brew.sh/formula/fpc, FPC
  3.2.2, Apple Silicon bottles available). Caveat for reference outputs: FPC's
  `Integer` is 16-bit only in TP mode, its `Real` is a 64-bit double, and
  `Crt` output is terminal-based, so plain-stdout tests are the comparable
  part.
- **FPC RTL reference** (procedure lists we match):
  Crt https://www.freepascal.org/docs-html/rtl/crt/index-5.html,
  Dos https://www.freepascal.org/docs-html/rtl/dos/index-5.html,
  Graph https://www.freepascal.org/docs-html/rtl/graph/index-5.html,
  System https://www.freepascal.org/docs-html/rtl/system/index-5.html.
- **FPC test suite** (https://gitlab.com/freepascal.org/fpc/source/-/tree/main/tests,
  layout in `tests/readme.txt`): directories `test/` (systematic tests by
  feature, e.g. `test/tstring*.pp`, `test/tset*.pp`, `test/tfile*.pp`), `tbs/`
  (bug tests that must compile and run), `tbf/` (must fail), `webtbs/`/
  `webtbf/` (bug-tracker cases), `units/`. Files are `t*.pp`, a test "returns
  0 on success and any other value on failure", and header directives like
  `{ %OPT=-Mtp }`, `{ %RESULT=201 }`, `{ %NORUN }`, `{ %INTERACTIVE }`,
  `{ %GRAPH }` describe how to run it. Takeaways for our harness: self-checking
  tests with an exit code are more robust than golden stdout for language
  semantics; we will use both. We don't copy FPC's tests (or any of its code):
  FPC is GPL and this project is MIT (decided 2026-10-28). Running `fpc -Mtp`
  as an external program to produce expected output is fine, and is what
  `tests/gen_expected.sh` does.
- **Testing FPC wiki**: https://wiki.lazarus.freepascal.org/Testing_FPC and
  https://wiki.lazarus.freepascal.org/Compiler_test_suite.

## 4. Is there an official Turbo Pascal spec or open-source implementation?

- There is no formal standard for Turbo Pascal. The specification is
  Borland's manual: **Turbo Pascal Version 7.0 Language Guide (1992)**, freely
  hosted at
  https://turbopascal.org/wp-content/uploads/Turbo_Pascal_Version_7.0_Language_Guide_1992.pdf
  (chapters on tokens, types, expressions, statements, units, objects, the
  System/Crt/Dos/Graph units, and the runtime error codes). The TP 5.0 and 6.0
  User's Guides are on bitsavers mirrors
  (https://archive.decromancer.ca/bitsavers.org/pdf/borland/turbo_pascal/).
- Borland's compiler was never open-sourced. Borland/Embarcadero's "Antique
  Software" museum gave away TP 1.0, 3.02 and 5.5 for personal use
  (https://winworldpc.com/product/turbo-pascal/5x lists the 5.5 package; the
  original `tp55.zip` links at embarcadero/codegear are dead, archive.org has
  mirrors). TP 5.5 runs under DOSBox on macOS, which is a way to get
  ground-truth outputs for the examples if ever needed.
- Open-source reimplementations: Free Pascal (above) is the practical one.
  **TPC16** (https://www.softpedia.com/get/Programming/Coding-languages-Compilers/TPC16-Compiler-Source-Code.shtml)
  is a from-scratch TP 7.0-compatible compiler written in TP that produces
  identical 16-bit code; interesting reading for TP semantics, not a tool we
  need. GNU Pascal (gpc) also had a Borland mode but is unmaintained.
- ISO 7185 (Standard Pascal, 1990) is free as an unofficial OCR copy at
  https://www.standardpascal.org/iso7185.html (and PDFs at
  https://www.cs.bilkent.edu.tr/~guvenir/courses/CS315/iso7185pascal.pdf).
  ISO 10206 (Extended Pascal) at
  https://www.infosun.fim.uni-passau.de/cl/lehre/sips1-ss06/Uebung/iso10206.pdf.
  Useful for the exact grammar (EBNF in section 6) even though we implement
  the Borland superset.
- The **Pascal Validation Suite** (Wichmann & Sale, Pascal News 16, 1979;
  https://bitsavers.mirrorservice.org/magazines/Pascal_News/16_Pascal_News_Oct79.pdf)
  is the historical ISO conformance suite; Scott Moore OCR'd it for Pascal-P5
  (https://sourceforge.net/projects/pascalp5/). It tests ISO features we do
  not target (conformant arrays, `get`/`put`), but its "deviance" and
  "quality" program lists are a good checklist of corner cases (evaluation
  order, `for` loop bounds, `case` without match).

## 5. Standard library interfaces to match

- **Turbo Pascal `System` unit** is the one to match for Pascrust's
  `pascrust_runtime::system`. The TP 7.0 Language Guide chapter 1 of the
  "library reference" lists it; FPC's System unit documentation is the same
  interface plus extensions. The core TP 5.5/7.0 `System` surface, grouped:
  - I/O: `Assign Reset Rewrite Append Close Read Readln Write Writeln Eof
    Eoln SeekEof SeekEoln Seek FilePos FileSize Erase Rename Truncate Flush
    IOResult BlockRead BlockWrite ChDir MkDir RmDir GetDir SetTextBuf`
  - Strings: `Length Copy Pos Delete Insert Concat Str Val UpCase`
  - Ordinal/arith: `Ord Chr Succ Pred Inc Dec Odd Abs Sqr Sqrt Sin Cos
    ArcTan Ln Exp Trunc Round Int Frac Pi Random Randomize Hi Lo Swap`
  - Memory: `New Dispose GetMem FreeMem Mark Release MaxAvail MemAvail SizeOf
    Addr Ptr Seg Ofs CSeg DSeg SPtr FillChar Move`
  - Control: `Exit Halt RunError ParamCount ParamStr ExitProc ExitCode
    ErrorAddr`
  - Variables: `Input Output RandSeed MaxInt MaxLongInt`
- **TP runtime error codes** (Language Guide appendix): 200 division by zero,
  201 range check, 202 stack overflow, 203 heap overflow, 204 invalid pointer,
  205 floating overflow, 207 invalid floating operation, 100 disk read error,
  101 disk write, 102 file not assigned, 103 file not open, 104/105 not open
  for input/output, 106 invalid numeric format, 2 file not found, 3 path not
  found, 4 too many open files, 5 access denied. Pascrust's `run()` prints
  these codes.
- **ISO 7185 required procedures** (`6.6.5`, `6.6.6`): `rewrite reset put get
  read readln write writeln page new dispose pack unpack abs sqr sin cos exp
  ln sqrt arctan trunc round ord chr succ pred odd eof eoln`. Everything
  except `page`, `pack`, `unpack`, `get`/`put` is a subset of TP's `System`,
  so matching TP covers ISO programs too.
- The **Crt/Dos/Graph** procedure lists in §3 above are the targets for the
  `crt`/`dos`/`graph` runtime features; the FPC docs double as the spec for
  argument types and constants (`FillPatternType = array[1..8] of Byte`,
  `WindMin`/`WindMax` packing `x` in the low byte and `y` in the high byte,
  `TextAttr` = `fg + 16*bg + 128*blink`, BGI colour numbers 0..15, line styles
  0..4, text justification 0..2).

## 6. Sources

- p2c README: https://github.com/FranklinChen/p2c/blob/master/README
- p2c options with comments: https://raw.githubusercontent.com/FranklinChen/p2c/master/src/sys.p2crc
- p2c runtime header: https://raw.githubusercontent.com/FranklinChen/p2c/master/src/p2c.h
- p2c man page: https://raw.githubusercontent.com/FranklinChen/p2c/master/src/p2c.man
- Gillespie on nested procedures (1990): https://www.tuhs.org/Usenet/comp.lang.c/1990-March/032197.html
- Gillespie's announcement (1989): https://compilers.iecc.com/comparch/article/89-11-022
- ferropascal: https://github.com/sunsided/ferropascal
- Free Pascal tests readme: https://gitlab.com/freepascal.org/fpc/source/-/raw/main/tests/readme.txt
- FPC Crt/Dos/Graph docs: links in §3
- TP 7.0 Language Guide PDF: https://turbopascal.org/wp-content/uploads/Turbo_Pascal_Version_7.0_Language_Guide_1992.pdf
- Turbo Pascal 5.5 package info: https://winworldpc.com/product/turbo-pascal/5x
- ISO 7185 HTML: https://www.standardpascal.org/iso7185.html
- Pascal Validation Suite (Pascal News 16): https://bitsavers.mirrorservice.org/magazines/Pascal_News/16_Pascal_News_Oct79.pdf
- crossterm 0.29: https://docs.rs/crate/crossterm/latest ; minifb 0.28: https://docs.rs/crate/minifb/latest
- Homebrew fpc: https://formulae.brew.sh/formula/fpc
