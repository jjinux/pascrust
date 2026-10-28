program VirtualSelf;
{ A static method that calls a virtual one runs the version for the object
  it's really called on. }
type
  TShape = object
    x, y: Integer;
    constructor Init(ax, ay: Integer);
    procedure Draw; virtual;
    procedure Show;
    function Name: string; virtual;
  end;
  TCircle = object(TShape)
    radius: Integer;
    constructor Init(ax, ay, r: Integer);
    procedure Draw; virtual;
    function Name: string; virtual;
  end;
  TRing = object(TCircle)
    procedure Draw; virtual;
  end;

var
  drawn: Integer;
  s: TShape;
  c: TCircle;
  r: TRing;

constructor TShape.Init(ax, ay: Integer);
begin
  x := ax;
  y := ay;
end;

procedure TShape.Draw;
begin
  drawn := drawn + 1;
  WriteLn('shape at ', x, ',', y);
end;

procedure TShape.Show;
begin
  Write(Name, ': ');
  Draw;
end;

function TShape.Name: string;
begin
  Name := 'shape';
end;

constructor TCircle.Init(ax, ay, r: Integer);
begin
  TShape.Init(ax, ay);
  radius := r;
end;

procedure TCircle.Draw;
begin
  drawn := drawn + 1;
  WriteLn('circle at ', x, ',', y, ' radius ', radius);
end;

function TCircle.Name: string;
begin
  Name := 'circle';
end;

procedure TRing.Draw;
begin
  Write('ring, inside a ');
  inherited Draw;
end;

procedure Paint(var shape: TShape);
begin
  shape.Draw;
end;

begin
  drawn := 0;
  s.Init(1, 2);
  c.Init(3, 4, 5);
  r.Init(6, 7, 8);
  s.Show;
  c.Show;
  r.Show;
  Paint(s);
  Paint(c);
  Paint(r);
  WriteLn(drawn, ' drawn');
end.
