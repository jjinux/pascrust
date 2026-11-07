{ The forms of goto Ruscal translates: back (a loop), forward past an
  if (a block), crossing jumps that nest once a block starts earlier, a
  numbered label inside repeat, out of a case arm, and in a procedure. }
program GotoForms;
var
   n, i: Integer;
label
   10, Top, Skip, Done, Inside;
procedure Count;
   label
      Next;
   var
      k: Integer;
   begin
      k := 0;
      Next:
      Inc(k);
      if k < 3 then
         goto Next;
      WriteLn('count ', k);
   end;
begin
   n := 0;
   Top:
   Inc(n);
   if n mod 2 = 0 then
      goto Skip;
   WriteLn('odd ', n);
   Skip:
   if n < 5 then
      goto Top;
   i := 0;
   repeat
      Inc(i);
      if i = 2 then
         goto 10;
      WriteLn('i ', i);
      10:
   until i >= 3;
   case n of
      5: begin
         WriteLn('five');
         goto Done;
      end;
   end;
   WriteLn('not reached');
   Done:
   Count;
   n := 0;
   Inside:
   Inc(n);
   if n < 2 then goto Inside;
   WriteLn('n ', n);
end.
