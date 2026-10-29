program GraphImageErrors;
{ GetImage and PutImage take a variable's bytes: not a value, not a file. }
uses Graph;
var
  t: Text;
begin
  GetImage(0, 0, 9, 9, 42);
  PutImage(0, 0, t, NormalPut);
  PutImage(0, 0, ImageSize(0, 0, 1, 1), NormalPut);
end.
