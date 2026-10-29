program ObjectsByValue;
{ A descendant object assigned or passed by value where an ancestor is
  expected: its ancestor part is copied. }
type
  TShape = object
    x: Integer;
    procedure Show;
  end;
  TCircle = object(TShape)
    r: Integer;
  end;
  PCircle = ^TCircle;
var
  s: TShape;
  c: TCircle;
  pc: PCircle;
procedure TShape.Show; begin WriteLn('shape ', x) end;
procedure ShowCopy(sh: TShape); begin sh.Show end;
begin
  c.x := 7;
  c.r := 3;
  s := c;
  s.Show;
  ShowCopy(c);
  New(pc);
  pc^.x := 9;
  s := pc^;
  s.Show;
end.
