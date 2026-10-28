program SystemQueries;
{ SizeOf (with Turbo Pascal's sizes), High, Low, Swap and Assigned.
  Self-checking: prints each failed check, then a count. (Free Pascal's
  sizes differ, e.g. its Real is 8 bytes, so this .out is written by hand.) }
type
  Color = (Red, Green, Blue);
  Digit = 0..9;
  Point = record
    x, y: Integer;
  end;
  Person = record
    name: string[20];
    age: Byte;
    height: Real;
    born: LongInt;
  end;
  Grid = array[1..4, 1..5] of Integer;
  Letters = array['a'..'z'] of Boolean;
  PPoint = ^Point;

const
  GridBytes = SizeOf(Grid);
  LastDigit = High(Digit);

var
  checks, failures: Integer;
  g: Grid;
  s: string[30];
  counts: array[Color] of Integer;
  p: PPoint;
  w: Word;
  i: Integer;
  small: array[0..LastDigit] of Char;

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
  Check(SizeOf(Integer) = 2, 'SizeOf(Integer)');
  Check(SizeOf(LongInt) = 4, 'SizeOf(LongInt)');
  Check(SizeOf(Byte) = 1, 'SizeOf(Byte)');
  Check(SizeOf(Char) = 1, 'SizeOf(Char)');
  Check(SizeOf(Boolean) = 1, 'SizeOf(Boolean)');
  Check(SizeOf(Real) = 6, 'SizeOf(Real)');
  Check(SizeOf(Color) = 1, 'SizeOf(Color)');
  Check(SizeOf(string) = 256, 'SizeOf(string)');
  Check(SizeOf(s) = 31, 'SizeOf(string[30])');
  Check(SizeOf(Point) = 4, 'SizeOf(Point)');
  Check(SizeOf(Person) = 21 + 1 + 6 + 4, 'SizeOf(Person)');
  Check(SizeOf(g) = 40, 'SizeOf(Grid)');
  Check(GridBytes = 40, 'SizeOf in a constant');
  Check(SizeOf(p) = 4, 'SizeOf(pointer)');
  Check(High(g) = 4, 'High(array)');
  Check(Low(Letters) = 'a', 'Low(array of char index)');
  Check(High(Letters) = 'z', 'High(array of char index)');
  Check(High(counts) = Blue, 'High(enum-indexed array)');
  Check(Low(Color) = Red, 'Low(Color)');
  Check(High(Integer) = 32767, 'High(Integer)');
  Check(Low(Integer) = -32768, 'Low(Integer)');
  Check(High(Byte) = 255, 'High(Byte)');
  Check(High(Char) = #255, 'High(Char)');
  Check(High(Boolean), 'High(Boolean)');
  Check(High(Digit) = 9, 'High(subrange)');
  Check(High(small) = 9, 'array bound from High');
  Check(High(s) = 30, 'High(string[30])');
  Check(Low(s) = 0, 'Low(string)');
  w := $1234;
  Check(Swap(w) = $3412, 'Swap(Word)');
  i := $0102;
  Check(Swap(i) = $0201, 'Swap(Integer)');
  p := nil;
  Check(not Assigned(p), 'Assigned(nil)');
  New(p);
  Check(Assigned(p), 'Assigned(p)');
  Dispose(p);
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
