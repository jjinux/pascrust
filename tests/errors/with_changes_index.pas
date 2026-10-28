program WithIndex;
{ The body changes i, so a[i] would mean a different element at each use. }
type
  Point = record
    x, y: Integer;
  end;
var
  a: array[1..3] of Point;
  i: Integer;
  r: Integer;
begin
  i := 1;
  with a[i] do
  begin
    x := 5;
    i := 2;
    y := 6;
  end;
  with r do
    r := 1;
end.
