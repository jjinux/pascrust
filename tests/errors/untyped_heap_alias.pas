{ Two untyped var arguments from one heap record: each one's bytes run
  on to the end of the record, so they overlap even for two fields. Two
  typed fields are fine. }
program UntypedHeapAlias;
type
  Pair = record
    x, y: Word;
  end;
  PPair = ^Pair;
var
  pp: PPair;

procedure Swap(var a, b: Word);
var
  t: Word;
begin
  t := a;
  a := b;
  b := t;
end;

procedure Both(var x, y; n: Integer);
begin
  Move(x, y, n);
end;

begin
  New(pp);
  Swap(pp^.x, pp^.y);
  Both(pp^.y, pp^.x, 2);
end.
