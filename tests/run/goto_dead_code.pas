{ Statements after a goto or Exit never run (old debugging code, say):
  the Rust keeps them, commented out, since Rust warns about code it
  can't reach. A goto among them needs no block. }
program GotoDeadCode;
var
   s: string;
   n: Integer;
label
   Done;

procedure Early;
   begin
      WriteLn('early');
      Exit;
      WriteLn('never');
   end;

begin
   n := 1;
   goto Done;
   s := 'debug';
   WriteLn('debug ', s,
      ' still debugging');
   goto Done;
   Done:
   Early;
   WriteLn('end ', n);
end.
