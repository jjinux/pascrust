{ Two labels on one statement (`L1: L2:`), a labeled `begin ... end`
  that jumps back to itself (a main loop), and a procedural field called
  with arguments inside its object's method. }
program GotoLabelsAndFields;
type
   TProc = procedure(x: Integer);
   TCounter = object
      Show: TProc;
      n: Integer;
      procedure Run;
   end;
var
   n: Integer;
   C: TCounter;
label
   Main, L1, L2;

procedure Print(x: Integer);
   begin
      WriteLn('show ', x);
   end;

procedure TCounter.Run;
   begin
      Show(n + 1);
   end;

begin
   n := 0;
   if n = 0 then
      goto L2;
   WriteLn('skipped');
   L1: L2: WriteLn('here ', n);
   Inc(n);
   if n < 2 then
      goto L1;
   n := 0;
   Main: begin
      Inc(n);
      WriteLn(n);
      if n = 3 then
      begin
         C.Show := Print;
         C.n := 41;
         C.Run;
         Halt;
      end;
      goto Main;
   end;
end.
