program DivOverflow;
{ The most negative Integer divided by -1 doesn't fit an Integer. TP's
  16-bit divide trapped, which it reported as runtime error 200, as for
  division by zero. (Free Pascal divides in a wider type and prints
  32768, so this .out is written by hand.) }
var
  i, j: Integer;
begin
  i := -32767 - 1;
  j := -1;
  WriteLn('dividing');
  WriteLn(i div j);
  WriteLn('not reached');
end.
