unit Shapes;
{ The root of an object hierarchy with a virtual method, for
  virtual_units.pas, which declares a descendant. }
interface
type
  PShape = ^TShape;
  TShape = object
    x: Integer;
    constructor Init(ax: Integer);
    procedure Draw; virtual;
    procedure Show;
  end;

procedure ShowAll(var shape: TShape);

implementation

constructor TShape.Init(ax: Integer);
begin
  x := ax;
end;

procedure TShape.Draw;
begin
  WriteLn('shape ', x);
end;

procedure TShape.Show;
begin
  Write('show: ');
  Draw;
end;

procedure ShowAll(var shape: TShape);
begin
  shape.Show;
end;

end.
