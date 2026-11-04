{ A goto out of a goto loop nested in another; a labeled final Exit; a
  repeat body that always leaves; while True. No unreachable code and no
  loop that never ends. }
program GotoNestedLoops;
label
   Main, Finish, Inner;
var
   i, j, k: Integer;

function F(N: Integer): Integer;
   label
      Fin;
   begin
      F := N;
      if N > 5 then
         goto Fin;
      F := N * 2;
      Fin:
      Exit;
   end;

procedure P;
   begin
      WriteLn('p');
      Exit; { all done }
   end;

begin
   i := 0;
   Main:
   i := i + 1;
   j := 0;
   Inner:
   j := j + 1;
   if (i = 2) and (j = 2) then
      goto Finish;
   if j < 3 then
      goto Inner;
   WriteLn('i=', i, ' j=', j);
   goto Main;
   Finish:
   WriteLn('done ', i, ' ', j);
   k := 0;
   repeat
      Inc(k);
      Break;
   until False;
   while True do
   begin
      Inc(k);
      if k > 3 then
         Break;
   end;
   WriteLn(F(3), ' ', F(7), ' ', k);
   P;
end.
