{ This program checks comment placement. }
program CommentsPlacement;
{ Where comments land in the Rust: after an end, before an else if,
  before until, on loop variables, on declarations with no Rust of
  their own. }
uses CmtUnit; { for Ready }

type
  TCount = Integer;   { an alias makes no Rust }
  Counter = object
    total: Integer;
    { adds one }
    procedure Bump;
  end;

var
  c: Counter;
  t: TCount;
  x, y: Integer;   { both coordinates }
  n: Integer;

procedure Later(k: Integer); forward; { body below }

{ the real Later }
procedure Later(k: Integer);
begin
  x := x + k;
end;

procedure Counter.Bump;
begin
  Inc(total);
end;

procedure Count;
var {Counts to three.}
  i: Integer;   { the counter }
begin
  for i := 1 to 3 do
    Write(i);
  WriteLn;
end;

procedure Foo;
begin
  x := 1;
end; { of Foo }

begin
  n := 0; {}
  Foo;
  Later(2 * { twice } 0);
  Count;
  if x > 0 then
  begin
    y := 1;
  end { then branch }
  else
  begin
    y := 2;
  end; { else branch }
  while n < 2 do
  begin
    Inc(n);
  end; { while loop }
  begin
    Inc(n);
  end; { a plain block }
  if x = 2 then
    y := 3
  { before else if }
  else if x = 1 then
    y := y + 10;
  repeat
    Inc(n);
    { about to test }
  until n > 4; { loop done }
  c.total := 0;
  c.Bump;
  t := c.total;
  WriteLn(x, ' ', y, ' ', n, ' ', Ready, ' ', t);
end. { end of program }
