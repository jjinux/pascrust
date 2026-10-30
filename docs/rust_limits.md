# Rewriting code that Rust can't express

PascRust translates Turbo Pascal into safe Rust, and a few things that are
fine in Pascal break one of Rust's rules. Rust allows only one mutable
reference to a variable at a time, and PascRust can't point into an
ordinary variable. When a program runs into one of these, PascRust stops
with an error that links to the section here explaining it. Each section
shows the Pascal that's refused and a rewrite that means the same and that
PascRust translates. The rewrites are plain Turbo Pascal, so the program
still compiles in TP.

<a id="two-var-arguments"></a>

## Two `var` arguments in one variable

```text
error: `a` is passed by `var` more than once in this call to `Swap`; Rust
doesn't allow two mutable references to the same variable (Rust can't tell
that two elements of one array are different)
```

A `var` parameter is a mutable reference in Rust, and Rust refuses two
mutable references into one variable at once. It borrows a whole array to
reach one element, so `a[i]` and `a[j]` count as the same place even when
`i <> j`. The same goes for two fields of one record kept in a variant
part's bytes, and for two `var` arguments in one heap record (`p^.a` and
`p^.a`).

Refused:

```pascal
procedure Swap(var x, y: Integer);
var
  t: Integer;
begin
  t := x;
  x := y;
  y := t;
end;
...
Swap(a[i], a[j]);
```

Rewrite: pass the array once, and the indexes.

```pascal
type
  TArr = array[1..5] of Integer;

procedure SwapItems(var items: TArr; i, j: Integer);
var
  t: Integer;
begin
  t := items[i];
  items[i] := items[j];
  items[j] := t;
end;
...
SwapItems(a, i, j);
```

For two fields of one record, pass the record (and say which fields in
another parameter if needed), or copy one field into a local variable,
pass the local, and copy it back after the call.

<a id="var-and-global"></a>

## A `var` argument that the routine also uses as a global

```text
error: `count` is passed by `var` to `Add`, which also uses `count`
directly; Rust doesn't allow two references to the same variable when one
of them is mutable
```

PascRust passes the globals a routine uses as parameters too, so `count`
would arrive twice: once as the `var` parameter, and once as the global
that `Add` (or a routine it calls) uses.

Refused:

```pascal
var
  count: Integer;

procedure Note;
begin
  Inc(count);
end;

procedure Add(var n: Integer; k: Integer);
begin
  Note;
  n := n + k;
end;
...
Add(count, 5);
```

Rewrite: a routine that changes a global can use the global itself, rather
than take it as a `var` parameter.

```pascal
procedure AddToCount(k: Integer);
begin
  Note;
  count := count + k;
end;
...
AddToCount(5);
```

If the routine must also work on other variables, copy the global into a
local variable, pass the local, and copy it back after the call.

<a id="with-selector"></a>

## A `with` statement that changes its own record's selector

```text
error: the `with` body changes `i`, which chooses this record; PascRust
writes the record out at every use, so it would pick a different one
```

Rust has no `with`, so PascRust writes `items[i].Qty` out at each use. In
Pascal, `with items[i] do` picks the record once, so a body that changes
`i` keeps working on the first one; written out, it would move to another.

Refused:

```pascal
with items[i] do
begin
  Qty := 0;
  Inc(i);
end;
```

Rewrite: choose the record with a variable the body doesn't change.

```pascal
chosen := i;
with items[chosen] do
begin
  Qty := 0;
  Inc(i);
end;
```

<a id="pchar-into-array"></a>

## Keeping a `PChar` into a `Char` array

```text
error: this keeps a PChar into a Char array, which PascRust can't do (it
lends a PChar to an array only for one statement); use a PChar from StrNew
or GetMem instead
```

PascRust can't point into an ordinary array, so where a zero-based `Char`
array is used as a `PChar`, the statement gets a copy of its characters,
written back after it (see [the architecture](architecture.md), the
`PChar` part). A `PChar` that outlives the statement would point at the
copy, not the array: `P := A`, `P := StrScan(A, '.')`, returning one, or
passing the array to a routine that keeps its `PChar` parameter.

Refused:

```pascal
var
  buf: array[0..79] of Char;
  p: PChar;
...
p := buf;
```

Rewrite: keep a `PChar` to memory of its own, from `StrNew` (a copy) or
`GetMem`; or keep the index into the array instead of a pointer.

```pascal
p := StrNew(buf);
...
StrDispose(p);
```

<a id="variant-with-pointer"></a>

## A tagless variant part with a pointer, stored as bytes

```text
error: `Node` has a variant part without a tag field that holds a pointer
(or a file), so PascRust keeps each variant separately and can't tell
which one to store as bytes
```

A variant part without a tag field keeps Turbo Pascal's bytes, so that
reading one variant as another works, but a pointer has no bytes in
PascRust. Such a record keeps its variants separately, and then a file (or
`Move`, or an untyped parameter) can't know which variant to store.

Refused:

```pascal
type
  PNode = ^Node;
  Node = record
    case Integer of
      0: (Next: PNode);
      1: (Value: LongInt);
  end;
var
  f: file of Node;
```

Rewrite: give the variant part a tag field, which says which variant is in
use (a stored pointer reads back as `nil`, as it would from a TP file in a
later run).

```pascal
type
  Node = record
    case IsLink: Boolean of
      True: (Next: PNode);
      False: (Value: LongInt);
  end;
```
