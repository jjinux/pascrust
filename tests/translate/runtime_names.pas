program RuntimeNames;
{ Type names that the runtime also uses (PSet, PArray, Ptr, PText,
  PFile) are renamed in the Rust, so a program can use them too. }
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
begin
  New(s);
  s^ := ['a', 'b'];
  a[2] := 7;
  New(p);
  p^ := a[2] * 2;
  f.name := 'notes.txt';
  f.size := p^;
  WriteLn('b' in s^, ' ', a[2], ' ', p^, ' ', f.name, ' ', f.size);
  Dispose(s);
  Dispose(p);
end.
