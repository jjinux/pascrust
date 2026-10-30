program PackedStringErrors;
{ A string constant must fill a packed string exactly; a zero-based Char
  array takes a shorter one, but not a longer one. }
var
  name: array[1..5] of Char;
  z: array[0..3] of Char;
begin
  name := 'Ab';
  z := 'abcde';
end.
