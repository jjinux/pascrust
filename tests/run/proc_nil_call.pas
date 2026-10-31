{ Calling a nil procedural variable: runtime error 216, as in
  protected-mode TP 7 (and Free Pascal). }
program ProcNilCall;
type
   Action = procedure;
var
   Go: Action;

procedure Hello;
   begin
      WriteLn('hello');
   end;

begin
   Go := Hello;
   Go;
   Go := nil;
   WriteLn(Assigned(Go));
   Go;
   WriteLn('not reached');
end.
