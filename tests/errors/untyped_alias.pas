{ An untyped var parameter's bytes run on to the end of the whole
  variable, so two such arguments in one variable, or one plus a global
  the routine uses, are two mutable borrows of it. }
program UntypedAlias;
type
  TArr = array[1..4] of Byte;
var
  a: TArr;
  g: Word;

procedure Both(var x, y; n: Integer);
begin
  Move(x, y, n);
end;

procedure Mixed(var x; var y: Byte);
begin
  y := 1;
end;

procedure Clear(var x);
begin
  FillChar(x, 2, 0);
  g := 1;
end;

begin
  Both(a[1], a[2], 2);
  Mixed(a, a[3]);
  Clear(g);
end.
