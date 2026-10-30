{ A conversion or a shift on the left of a comparison: Rust reads
  `i as u16 < w` as the start of generic arguments, so the cast needs
  parentheses there (and only there). }
program CastCompare;
var
  i: Integer;
  w: Word;
  b: Byte;
begin
  i := 80;
  w := 100;
  b := 3;
  if i shr 1 < 50 then
    WriteLn('shr');
  if Word(i) < w then
    WriteLn('Word');
  if Byte(i + 200) <= 24 then
    WriteLn('Byte');
  if not Boolean(b - 3) then
    WriteLn('Boolean');
  WriteLn(Word(i) + w, ' ', ShortInt(b * 50) < 0);
end.
