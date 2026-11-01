{ The jumps PascRust can't make into blocks or loops, and labels used
  wrongly. }
program GotoErrors;
var
   i: Integer;
label
   Inside, Twice, Unused, A, B, X, Y;
begin
   goto Inside;
   for i := 1 to 3 do
   begin
      Inside:
      WriteLn(i);
   end;
   Twice:
   WriteLn('one');
   Twice:
   WriteLn('two');
   goto Nowhere;
   goto Unused;
   while i < 10 do
   begin
      if i = 5 then
         goto A;
      if i = 6 then
         Break;
      Inc(i);
      A:
   end;
   B:
   if i = 1 then
      goto A;
   goto B;
   goto X;
   Y:
   WriteLn('y');
   X:
   WriteLn('x');
   goto Y;
end.
