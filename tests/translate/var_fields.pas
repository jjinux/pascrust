program VarFields;
{ Two different fields of one record can be passed as two var arguments. }
type
  Point = record
    x, y: Integer;
  end;
  Circle = record
    center: Point;
    radius: Integer;
  end;
  Shape = object
    center: Point;
    radius: Integer;
    procedure Init;
  end;
  PPoint = ^Point;

var
  p: Point;
  c: Circle;
  s: Shape;
  q: PPoint;

procedure SetTwo(var a, b: Integer);
begin
  a := 1;
  b := 2;
end;

function Swapped(var a, b: Integer): Boolean;
var
  t: Integer;
begin
  t := a;
  a := b;
  b := t;
  Swapped := a <> b;
end;

procedure Fill(var pt: Point);
begin
  SetTwo(pt.x, pt.y);
end;

procedure Shape.Init;
begin
  SetTwo(center.x, radius);
end;

procedure Local;
var
  r: Circle;
begin
  SetTwo(r.center.y, r.radius);
  WriteLn(r.center.y, ' ', r.radius);
end;

begin
  SetTwo(p.x, p.y);
  WriteLn(p.x, ' ', p.y);
  SetTwo(c.center.x, c.radius);
  WriteLn(c.center.x, ' ', c.radius);
  p.x := 0;
  Fill(p);
  WriteLn(p.x, ' ', p.y);
  s.Init;
  WriteLn(s.center.x, ' ', s.radius);
  Local;
  New(q);
  SetTwo(q^.x, q^.y);
  WriteLn(q^.x, ' ', q^.y);
  if Swapped(q^.x, q^.y) then
    WriteLn(q^.x, ' ', q^.y);
  Dispose(q);
end.
