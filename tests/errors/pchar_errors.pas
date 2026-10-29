program PCharErrors;
{ A PChar into a Char array can't outlive the statement: PascRust lends
  the array a copy of its characters for that statement only. }
uses Strings;
type
  TBuf = array[0..15] of Char;
var
  buf: TBuf;
  p, kept: PChar;
  s: string;

procedure Keep(x: PChar);
begin
  kept := x;
end;

procedure KeepLater(y: PChar);
var
  local: PChar;
begin
  local := y;
  Keep(local);
end;

function Echo(z: PChar): PChar;
begin
  Echo := z;
end;

procedure Look(w: PChar);
var
  copy: PChar;
begin
  copy := w;
  WriteLn(copy);
end;

begin
  p := buf;
  p := StrScan(buf, 'a');
  p := Echo(buf);
  Keep(buf);
  KeepLater(buf);
  Look(buf);
  WriteLn(Echo(buf));
  p := s;
  p := Echo(StrNew(buf));
end.
