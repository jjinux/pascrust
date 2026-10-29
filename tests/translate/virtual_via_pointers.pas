program VirtualViaPointers;
{ Through a pointer, the record's real type picks the method: when the
  record is passed to a var parameter of an ancestor type, and for methods
  first declared below the root of the hierarchy. }
type
  PShape = ^TShape;
  TShape = object
    x: Integer;
    constructor Init(ax: Integer);
    procedure Draw; virtual;
    procedure Show;
  end;
  PCircle = ^TCircle;
  TCircle = object(TShape)
    r: Integer;
    constructor Init(ax, ar: Integer);
    procedure Draw; virtual;
    function Area: Integer; virtual;
    procedure Report;
  end;
  PFilled = ^TFilled;
  TFilled = object(TCircle)
    function Area: Integer; virtual;
  end;

var
  shapes: array[1..3] of PShape;
  pc: PCircle;
  i: Integer;

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

constructor TCircle.Init(ax, ar: Integer);
begin
  TShape.Init(ax);
  r := ar;
end;

procedure TCircle.Draw;
begin
  WriteLn('circle ', x, ' radius ', r);
end;

function TCircle.Area: Integer;
begin
  Area := 3 * r * r;
end;

procedure TCircle.Report;
begin
  WriteLn('area ', Area);
end;

function TFilled.Area: Integer;
begin
  Area := inherited Area + 1;
end;

procedure Paint(var s: TShape);
begin
  s.Show;
end;

procedure Measure(var c: TCircle);
begin
  c.Report;
end;

begin
  shapes[1] := New(PShape, Init(1));
  shapes[2] := New(PCircle, Init(2, 3));
  shapes[3] := New(PFilled, Init(4, 5));
  for i := 1 to 3 do
    Paint(shapes[i]^);
  pc := New(PFilled, Init(6, 2));
  pc^.Report;
  Measure(pc^);
  WriteLn(pc^.Area);
end.
