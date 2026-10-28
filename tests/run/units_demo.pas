program UnitsDemo;
uses Geometry, Labels;
var
  p: Point;
begin
  p := MakePoint(3, 4);
  Tag(p, 'home');
  Tag(MakePoint(1, 2), 'work');
  writeln(Name, ' ', Sides, ' ', created);
end.
