program ConditionComments;
{ A loop condition whose translation needs a teaching comment (the first
  `shl` of the program) and temporaries: the comment goes before the loop,
  not inside the condition's block. Output from fpc -Mtp. }
var
   n, calls: Integer;

function Show(k: Integer): Integer;
   begin
      Inc(calls);
      Show := k;
   end;

begin
   n := 3;
   calls := 0;
   while (n > 0) and (Show(n) shl n > 0) do
      Dec(n);
   WriteLn(n, ' ', calls);
   repeat
      Inc(n);
   until Show(n) shl 2 > 8;
   WriteLn(n, ' ', calls);
end.
