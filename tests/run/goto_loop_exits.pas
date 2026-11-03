{ Break and Continue inside the statements a goto skips or repeats, for
  a for, while or repeat loop around them: the loop gets a label. }
program GotoLoopExits;
var
   i, j, found: Integer;
label
   Next, Next2, Again;
begin
   found := 0;
   for i := 1 to 5 do
   begin
      if i = 2 then
         goto Next;
      if i = 4 then
         Break;
      Inc(found, i);
      Next:
   end;
   WriteLn('for ', found, ' ', i);
   i := 0;
   while i < 10 do
   begin
      Inc(i);
      if Odd(i) then
         goto Next2;
      if i = 6 then
         Continue;
      WriteLn('even ', i);
      if i = 8 then
         Break;
      Next2:
   end;
   j := 0;
   repeat
      Inc(j);
      Again:
      if j = 3 then
         Break;
      if j < 2 then
      begin
         Inc(j);
         goto Again;
      end;
   until j > 10;
   WriteLn('repeat ', j);
end.
