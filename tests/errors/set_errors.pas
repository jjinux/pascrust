program SetErrors;
{ A set's elements need ordinals 0 to 255, and can't be mixed. }
type
  Color = (Red, Green);
var
  big: set of Integer;
  colors: set of Color;
  chars: set of Char;
begin
  colors := [Red, 'a'];
  Include(chars, Red);
  if 1 in chars then
    WriteLn;
end.
