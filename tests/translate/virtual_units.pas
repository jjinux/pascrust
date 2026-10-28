program VirtualUnits;
{ A descendant declared in the program of an object whose unit has the
  virtual method. }
uses Shapes;
type
  PCircle = ^TCircle;
  TCircle = object(TShape)
    radius: Integer;
    procedure Draw; virtual;
  end;

var
  c: TCircle;
  p: PShape;

procedure TCircle.Draw;
begin
  WriteLn('circle ', x, ' radius ', radius);
end;

begin
  c.Init(1);
  c.radius := 2;
  ShowAll(c);
  p := New(PCircle, Init(3));
  p^.Show;
  Dispose(p);
end.
