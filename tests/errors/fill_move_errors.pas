program FillMoveErrors;
{ FillChar and Move only in the safe, common cases. }
type
  Point = record
    x, y: Integer;
  end;
var
  p: Point;
  n: array[1..4] of Integer;
  b: array[1..4] of Byte;
begin
  FillChar(p, SizeOf(p), 1);
  FillChar(n, 3, 0);
  Move(n, b, SizeOf(n));
end.
