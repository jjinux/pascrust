program AnonRecords;
{ Variables of anonymous record types become structs named after the
  variable; two variables with the same name get `Point` and `Point2`. }
procedure First;
var
   point: record x, y: integer; end;
begin
   point.x := 1;
   point.y := 2;
   writeln('first: ', point.x, ',', point.y);
end;

procedure Second;
var
   point: record x, y, z: integer; end;
begin
   point.x := 3;
   point.y := 4;
   point.z := 5;
   writeln('second: ', point.x, ',', point.y, ',', point.z);
end;

begin
   First;
   Second;
end.
