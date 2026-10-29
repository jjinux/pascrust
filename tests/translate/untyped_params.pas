program UntypedParams;
{ Untyped var parameters: a routine sees the caller's variable as Turbo
  Pascal's bytes (TBytes(Data)[i]), can write them back (also when it
  leaves through Exit), pass them on, and read them as another type;
  Move and FillChar between different types; an argument that starts
  inside a variable (s[1], buf[3]). Output from fpc -Mtp. }
type
  TBytes = array[0..255] of Byte;
  Rec = record
    name: string[5];
    n: Integer;
  end;
var
  r, r2: Rec;
  s: string[10];
  w: Word;
  buf: array[1..8] of Byte;
  i: Integer;

procedure Dump(var Data; Size: Word);
var i: Integer;
begin
  for i := 0 to Size - 1 do
    Write(TBytes(Data)[i], ' ');
  WriteLn;
end;

procedure Zero(var Data; Size: Word);
var i: Integer;
begin
  for i := 0 to Size - 1 do
    TBytes(Data)[i] := 0;
end;

procedure Pass(var Data; Size: Word);
begin
  Dump(Data, Size);
end;

procedure SetSome(var Data; Count: Word);
var i: Integer;
begin
  for i := 0 to 7 do
  begin
    if i = Count then
      Exit;
    TBytes(Data)[i] := 9;
  end;
end;

function First(var Data): Rec;
begin
  First := Rec(Data);
end;

begin
  r.name := 'abc';
  r.n := 258;
  Dump(r, SizeOf(r));
  w := $1234;
  Pass(w, 2);
  s := 'hello';
  Dump(s[1], 3);
  for i := 1 to 8 do buf[i] := i;
  Zero(buf[3], 2);
  Dump(buf, 8);
  Move(r, buf, 8);
  Dump(buf, 8);
  FillChar(r, SizeOf(r), 0);
  WriteLn(r.n, ' ', Length(r.name));
  r2 := First(buf);
  WriteLn(r2.name, ' ', r2.n);
  SetSome(buf, 3);
  Dump(buf, 8);
  FillChar(r, SizeOf(r), 1);
  WriteLn(r.n, ' ', Length(r.name));
  FillChar(buf, 3, 0);
  Dump(buf, 8);
end.
