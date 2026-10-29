program RuntimeNames;
{ Type names that the runtime also uses (PSet, PArray, Ptr, PText,
  PFile), and the names of runtime functions the Rust calls behind the
  scenes (Random is random_real, a for loop over an enum uses
  enum_range), are renamed, so a program can use them too. }
type
  CharSet = set of Char;
  PSet = ^CharSet;
  PArray = array[1..3] of Integer;
  Ptr = ^Integer;
  PText = string[10];
  PFile = record
    name: PText;
    size: Integer;
  end;
var
  s: PSet;
  a: PArray;
  p: Ptr;
  f: PFile;
  c: (Red, Green, Blue);
  r: Real;

function RandomReal: Real;
begin
  RandomReal := 0.5;
end;

function EnumRange(n: Integer): Integer;
begin
  EnumRange := n * 2;
end;

procedure SetRandSeed(n: Integer);
begin
  WriteLn('my SetRandSeed ', n);
end;

begin
  New(s);
  s^ := ['a', 'b'];
  a[2] := 7;
  New(p);
  p^ := a[2] * 2;
  f.name := 'notes.txt';
  f.size := p^;
  WriteLn('b' in s^, ' ', a[2], ' ', p^, ' ', f.name, ' ', f.size);
  RandSeed := 1;
  SetRandSeed(2);
  r := Random;
  for c := Red to Blue do
    Write(Ord(c));
  WriteLn(' ', r < 1, ' ', RandomReal:3:1, ' ', EnumRange(3));
  Dispose(s);
  Dispose(p);
end.
