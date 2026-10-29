program DosRefused;
{ Routines that need real DOS are a clear error. }
uses Dos;
var r: Registers;
begin
  Intr($10, r);
  Keep(0);
end.
