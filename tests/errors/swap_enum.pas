program SwapEnum;
{ Swap takes an Integer or a Word; an enum constant isn't one. }
type
  Color = (Red, Green, Blue);
const
  K = Blue;
begin
  WriteLn(Swap(K));
  { one message, not two }
  WriteLn(Swap(K + 1));
end.
