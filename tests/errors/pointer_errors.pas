program PointerErrors;
{ An untyped Pointer's target has no type: p^ only goes where any
  variable's bytes can (an untyped var parameter, Move, BlockRead). }
var
  p: Pointer;
  n: Integer;
begin
  GetMem(p, 2);
  n := p^;
  WriteLn(p^);
  GetMem(p);
end.
