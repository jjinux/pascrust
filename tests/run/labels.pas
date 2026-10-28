unit Labels;
{ Used by units_demo.pas. Declares `Name` too: the program uses both units,
  and Turbo Pascal's rule is that the unit listed later wins. }

interface

uses Geometry;

function Name: string;
procedure Tag(p: Point; text: string);

implementation

function Name: string;
begin
  Name := 'labels';
end;

procedure Tag(p: Point; text: string);
begin
  write(text, ' at ');
  Describe(p);
end;

begin
  writeln('Labels initialized');
end.
