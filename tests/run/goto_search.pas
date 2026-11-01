program GotoSearch;
{ Look for a number in a grid, leaving both loops at once with a goto
  forward, and go back to try a smaller one with a goto back. }
const
   Size = 4;
var
   Grid: array[1..Size, 1..Size] of Integer;
   Row, Col, Target, Tries: Integer;
label
   Found, Again;
begin
   for Row := 1 to Size do
      for Col := 1 to Size do
         Grid[Row, Col] := Row * 10 + Col;
   Target := 64;
   Tries := 0;
   Again:
   Inc(Tries);
   for Row := 1 to Size do
      for Col := 1 to Size do
         if Grid[Row, Col] = Target then
            goto Found;
   WriteLn(Target, ' is not in the grid');
   Target := Target - 10;
   if Tries < 3 then
      goto Again;
   WriteLn('gave up');
   Exit;
   Found:
   WriteLn('found ', Target, ' at ', Row, ',', Col, ' after ', Tries, ' tries');
end.
