unit Geometry;
{ Used by units_demo.pas. }

interface

const
  Sides = 4;

type
  Point = record
    x, y: integer;
  end;

var
  created: integer;   { a unit global, counted by MakePoint }

function MakePoint(x, y: integer): Point;
procedure Describe(p: Point);
function Name: string;

implementation

function MakePoint(x, y: integer): Point;
var
  p: Point;
begin
  p.x := x;
  p.y := y;
  created := created + 1;
  MakePoint := p;
end;

procedure Describe(p: Point);
begin
  writeln('(', p.x, ', ', p.y, ')');
end;

function Name: string;
begin
  Name := 'geometry';
end;

begin
  created := 0;
  writeln('Geometry initialized');
end.
