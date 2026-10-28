program WriteEnum;
{ Turbo Pascal can't write an enumerated value. }
type
   Color = (Red, Green, Blue);
var
   c: Color;
begin
   c := Green;
   writeln(c);
end.
