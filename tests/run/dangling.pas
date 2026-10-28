program Dangling;
{ Using a pointer after its record was disposed is a runtime error (204),
  never undefined behaviour. }
type
  IntPtr = ^integer;
var
  a, b: IntPtr;
begin
  new(a);
  a^ := 42;
  b := a;
  dispose(a);
  writeln('disposed');
  writeln(b^);
end.
