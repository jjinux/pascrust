{ A labeled main loop with a goto out of it: the block the goto leaves
  goes around the loop, so leaving it leaves the loop too. }
program GotoMainExit;
label
   Main, Finish;
var
   n: Integer;
begin
   n := 0;
   Main: begin
      Inc(n);
      if n >= 3 then
         goto Finish;
      goto Main
   end;
   Finish:
   WriteLn('n=', n);
end.
