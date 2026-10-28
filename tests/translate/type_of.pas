program TypeOfTest;
{ TypeOf tells which object type something really is: through a pointer,
  a var parameter, or Self, it's the real object's type. }
type
  PShape = ^TShape;
  TShape = object
    constructor Init;
    procedure Draw; virtual;
    function IsCircle: Boolean;
  end;
  PCircle = ^TCircle;
  TCircle = object(TShape)
    procedure Draw; virtual;
  end;
  TRing = object(TCircle)
  end;

var
  s: TShape;
  c: TCircle;
  r: TRing;
  p, q: PShape;

constructor TShape.Init;
begin
end;

procedure TShape.Draw;
begin
  WriteLn('shape');
end;

function TShape.IsCircle: Boolean;
begin
  IsCircle := TypeOf(Self) = TypeOf(TCircle);
end;

procedure TCircle.Draw;
begin
  WriteLn('circle');
end;

procedure Check(var shape: TShape);
begin
  if TypeOf(shape) = TypeOf(TShape) then
    WriteLn('a plain shape')
  else if TypeOf(shape) = TypeOf(TCircle) then
    WriteLn('a circle')
  else
    WriteLn('something else');
end;

begin
  s.Init;
  c.Init;
  r.Init;
  Check(s);
  Check(c);
  Check(r);
  WriteLn(s.IsCircle, ' ', c.IsCircle, ' ', r.IsCircle);
  WriteLn(TypeOf(c) = TypeOf(TCircle), ' ', TypeOf(r) = TypeOf(TCircle));
  New(p, Init);
  q := New(PCircle, Init);
  WriteLn(TypeOf(p^) = TypeOf(q^), ' ', TypeOf(q^) = TypeOf(TCircle), ' ', TypeOf(p^) <> TypeOf(TCircle));
  Dispose(p);
  Dispose(q);
end.
