program AliasFields;
{ Different fields are fine, but overlapping places are two mutable
  references to the same storage. }
type
  Point = record
    x, y: Integer;
    v: array[1..2] of Integer;
  end;
  PPoint = ^Point;
var
  p: Point;
  q: PPoint;

procedure SetTwo(var a, b: Integer);
begin
end;

procedure Whole(var a: Point; var b: Integer);
begin
end;

begin
  SetTwo(p.x, p.y);
  SetTwo(q^.x, q^.y);
  Whole(p, p.x);
  SetTwo(p.v[1], p.v[2]);
  SetTwo(q^.x, q^.x);
end.
