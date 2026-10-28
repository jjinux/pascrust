program RepeatContinue;
{ Continue in a repeat loop still tests the until condition. }
var
   i, j: integer;
begin
   i := 0;
   repeat
      i := i + 1;
      if i mod 2 = 0 then
         continue;
      if i = 9 then
         break;
      j := 0;
      repeat
         j := j + 1;
         if j = 2 then
            continue;
         write(i, ':', j, ' ');
      until j >= 3;
      writeln;
   until i >= 6;
   writeln('i = ', i);
end.
