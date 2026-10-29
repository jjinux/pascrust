program SetRange;
{ A value outside a set's element range is never in it: 300 isn't in a
  set of Byte, whether it's a constant or a variable. Self-checking.
  (Free Pascal cuts the variable down to a byte, 300 to 44, so this .out
  is built from the checks, by hand.) }
var
  checks, failures: Integer;
  s: set of Byte;
  i: Integer;
  w: Word;

procedure Check(ok: Boolean; what: string);
begin
  checks := checks + 1;
  if ok then
    WriteLn('ok: ', what)
  else
  begin
    failures := failures + 1;
    WriteLn('FAILED: ', what);
  end;
end;

begin
  checks := 0;
  failures := 0;
  s := [44, 200];
  i := 300;
  w := 256 + 200;
  Check(not (300 in s), 'a constant above 255 is not in a set of Byte');
  Check(not (i in s), 'an Integer variable above 255 is not in it either');
  Check(not (w in s), 'nor a Word variable');
  i := 44;
  Check(i in s, 'an Integer variable in range is');
  i := -1;
  Check(not (i in s), 'a negative value is not in a set of Byte');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
