program RecordLayout;
{ Typed files hold records in Turbo Pascal's own layout, the size SizeOf
  gives: a string[30] is 31 bytes, a LongInt 4, a Real 6 (TP's format), so
  files written by TP programs can be read. Self-checking. (Free Pascal
  pads records and writes 8-byte reals, so this .out is built from the
  checks, by hand.) }
type
  People = record
    name: string[30];
    ID: LongInt;
    Index: Real;
  end;
  Color = (Red, Green, Blue);
  Paint = record
    id: Integer;
    colors: set of Color;
  end;
var
  checks, failures: Integer;
  f: file of People;
  raw: file of Byte;
  p, q: People;
  b: Byte;
  i: Integer;
  bytes: array[1..41] of Byte;
  pf: file of Paint;
  brush, again: Paint;

procedure Check(ok: Boolean; what: string);
begin
  checks := checks + 1;
  if ok then
    WriteLn('ok: ', what)
  else
  begin
    failures := failures + 1;
    WriteLn('FAILED: ', what);
  end;
end;

begin
  checks := 0;
  failures := 0;
  Check(SizeOf(People) = 41, 'SizeOf(People) is 41');
  p.name := 'Shannon Behrens';
  p.ID := 1;
  p.Index := 2930802.0;
  Assign(f, 'people.dat');
  Rewrite(f);
  Write(f, p);
  Close(f);
  Assign(raw, 'people.dat');
  Reset(raw);
  Check(FileSize(raw) = 41, 'one record is 41 bytes on disk');
  for i := 1 to 41 do
    Read(raw, bytes[i]);
  Close(raw);
  Check((bytes[1] = 15) and (bytes[2] = Ord('S')), 'the name''s length byte, then its characters');
  Check((bytes[32] = 1) and (bytes[33] = 0), 'the LongInt after the 31 bytes of the name');
  Check((bytes[36] = 150) and (bytes[41] = $32), 'the Real in TP''s 6-byte format');
  Reset(f);
  Read(f, q);
  Close(f);
  Erase(f);
  Check((q.name = p.name) and (q.ID = 1) and (q.Index = 2930802.0), 'and it reads back');
  brush.id := 7;
  brush.colors := [Red, Blue];
  Assign(pf, 'paint.dat');
  Rewrite(pf);
  Write(pf, brush);
  Close(pf);
  Assign(raw, 'paint.dat');
  Reset(raw);
  Check((SizeOf(Paint) = 3) and (FileSize(raw) = 3), 'a set of three colours takes one byte');
  Close(raw);
  Reset(pf);
  Read(pf, again);
  Close(pf);
  Erase(pf);
  Check((again.id = 7) and (again.colors = [Red, Blue]), 'and reads back');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
