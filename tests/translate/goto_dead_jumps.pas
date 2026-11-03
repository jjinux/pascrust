{ Dead code after statements that always jump away: an if whose two
  branches both jump, a begin ... end ending in a goto; and routines
  ending in Exit. The Rust has no unreachable code (and the commented-out
  code keeps its nesting). }
program GotoDeadJumps;
label
   Outer, Done, Done2;
var
   i, n, x: Integer;

function F(n: Integer): Integer;
   begin
      F := n * 2;
      Exit;
   end;

procedure P;
   begin
      WriteLn('p');
      Exit
   end;

begin
   i := 0;
   Outer:
   Inc(i);
   if i < 3 then
      goto Outer
   else
      goto Done;
   WriteLn('never');
   Done:
   begin
      goto Done2
   end;
   WriteLn('never 2');
   case i of
      1: x := 1;
      2: begin
         x := 2;
         WriteLn('two');
      end;
   end;
   Done2:
   n := F(i);
   P;
   WriteLn(i, ' ', n);
end.
