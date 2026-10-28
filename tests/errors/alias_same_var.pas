program BadAlias;
var x: integer;
procedure Swap(var a, b: integer);
var t: integer;
begin t := a; a := b; b := t end;
begin
  Swap(x, x);
end.
