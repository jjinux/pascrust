program ConstantRange;
{ Constants that don't fit the variable they're stored in: TP says
  "Constant out of range". }
const
  Big = 70000;
var
  i: Integer;
  b: Byte;
begin
  i := 65535;
  i := Big;
  i := Swap(-1);
  b := 300;
  b := -1;
  for b := 0 to 256 do
    WriteLn(b);
end.
