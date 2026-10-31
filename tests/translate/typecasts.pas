program Typecasts;
{ Value typecasts: Type(x) converts between ordinal types. }
type
   Color = (Red, Green, Blue);
var
   i: integer;
   b: Boolean;
   c: Color;
begin
   i := 2;
   b := Boolean(i);
   c := Color(i - 1);
   writeln(b, ' ', Boolean(0), ' ', Ord(c), ' ', integer(Blue), ' ', Char(65), ' ', Byte('a'));
end.
