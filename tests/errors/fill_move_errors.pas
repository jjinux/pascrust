program FillMoveErrors;
{ FillChar and Move past the end of a variable: TP would have overwritten
  whatever came next in memory. }
var
  n: array[1..4] of Integer;
  b: array[1..4] of Byte;
begin
  Move(n, b, SizeOf(n));
  FillChar(b, 10, 0);
end.
