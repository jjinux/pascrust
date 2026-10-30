program AddrErrors;
{ `@` only takes a procedure's address (for SetIntVec), and an interrupt
  vector holds a Pointer. }
uses Dos;
var
  n: Integer;
  p: Pointer;
begin
  p := @n;
  SetIntVec($1C, n);
  GetIntVec($1C, n);
end.
