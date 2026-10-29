program BlockFileErrors;
{ Untyped files only go with BlockRead and BlockWrite, and a record size
  is only for them. }
var
  f: file;
  t: Text;
  g: file of Integer;
  n: Integer;
begin
  Write(f, n);
  Reset(g, 1);
  BlockRead(t, n, 1);
end.
