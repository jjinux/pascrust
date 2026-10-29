{ This program checks comment placement. }
program CommentsPlacement;
{ Where comments land in the Rust: after an end, before an else if,
  before until, on loop variables, on declarations with no Rust of
  their own. }
uses CmtUnit, CmtUnit2; { for Ready and Two }

const
  Primes: array[1..3] of Integer =
    (2, 3, 5);   { the first primes }

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

procedure Outer;

  procedure Later;   { a nested namesake }
  begin
  end;

begin
  Later;
end;

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
  i, j: Integer;   { the counters }
begin
  for i := 1 to 3 do
    Write(i);
  for j := 1 to 0 do
    Write(j);
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
  if x > 5 then begin
    y := 0;
  end else begin { otherwise }
    y := y + 0;
  end;
  begin end; { an empty block }
  case x of
    5: y := 0;
    { other values
      need nothing }
  end; { case }
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
  Outer;
  WriteLn(x, ' ', y, ' ', n, ' ', Ready, ' ', t, ' ', Primes[3] + Two);
end. { end of program }
