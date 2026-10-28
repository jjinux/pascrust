program RunErrorTest;
{ RunError stops the program with a runtime error of its own choosing.
  (Free Pascal prints its runtime errors on standard output, so this .out
  is written by hand.) }
begin
  WriteLn('before');
  RunError(42);
  WriteLn('not reached');
end.
