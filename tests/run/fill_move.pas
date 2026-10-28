program FillMove;
{ FillChar and Move in the common, safe cases. Self-checking. }
type
  Point = record
    x, y: Integer;
    name: string[10];
  end;

var
  checks, failures: Integer;
  bytes: array[1..10] of Byte;
  chars: array[0..4] of Char;
  flags: array[1..3] of Boolean;
  grid: array[1..3, 1..3] of Integer;
  p, q: Point;
  s: string[20];
  i, total: Integer;

procedure Check(ok: Boolean; what: string);
begin
  checks := checks + 1;
  if not ok then
  begin
    failures := failures + 1;
    WriteLn('FAILED: ', what);
  end;
end;

begin
  checks := 0;
  failures := 0;
  for i := 1 to 10 do
    bytes[i] := i;
  FillChar(bytes, SizeOf(bytes), 0);
  total := 0;
  for i := 1 to 10 do
    total := total + bytes[i];
  Check(total = 0, 'zero a byte array');
  FillChar(bytes, SizeOf(bytes), 7);
  Check((bytes[1] = 7) and (bytes[10] = 7), 'fill a byte array');
  FillChar(bytes[3], 4, 1);
  Check((bytes[2] = 7) and (bytes[3] = 1) and (bytes[6] = 1) and (bytes[7] = 7), 'fill part of a byte array');
  FillChar(chars, SizeOf(chars), '*');
  Check((chars[0] = '*') and (chars[4] = '*'), 'fill a char array');
  FillChar(flags, SizeOf(flags), 1);
  Check(flags[1] and flags[3], 'fill a boolean array');
  grid[2, 2] := 5;
  FillChar(grid, SizeOf(grid), 0);
  Check(grid[2, 2] = 0, 'zero a 2D array');
  p.x := 1;
  p.y := 2;
  p.name := 'abc';
  FillChar(p, SizeOf(p), 0);
  Check((p.x = 0) and (p.y = 0) and (p.name = ''), 'zero a record');
  s := 'hello';
  FillChar(s, SizeOf(s), 0);
  Check(Length(s) = 0, 'zero a string');
  FillChar(s[1], 5, '-');
  s[0] := Chr(5);
  Check(s = '-----', 'fill part of a string');
  p.x := 3;
  p.y := 4;
  p.name := 'pt';
  Move(p, q, SizeOf(p));
  Check((q.x = 3) and (q.y = 4) and (q.name = 'pt'), 'move a record');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
