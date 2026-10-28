program Procedures;
{ Value and var parameters, functions, recursion, forward declarations,
  nested procedures that use outer locals, and globals. }
var
  x, y: integer;
  counter: integer;

procedure Swap(var a, b: integer);
var
  t: integer;
begin
  t := a;
  a := b;
  b := t;
end;

function Fact(n: integer): longint;
begin
  if n <= 1 then
    Fact := 1
  else
    Fact := n * Fact(n - 1);
end;

function IsEven(n: integer): boolean; forward;

function IsOdd(n: integer): boolean;
begin
  if n = 0 then IsOdd := false else IsOdd := IsEven(n - 1);
end;

function IsEven(n: integer): boolean;
begin
  if n = 0 then IsEven := true else IsEven := IsOdd(n - 1);
end;

procedure Bump;
begin
  counter := counter + 1;
end;

procedure BumpTwice;
begin
  Bump;
  Bump;
end;

procedure Report(limit: integer);
var
  total: integer;

  procedure Add(n: integer);
  begin
    total := total + n;
  end;

  procedure AddUpTo(n: integer);
  begin
    if n > 0 then
    begin
      Add(n);
      AddUpTo(n - 1);
    end;
  end;

begin
  total := 0;
  AddUpTo(limit);
  writeln('total = ', total);
end;

function Twice(n: integer): integer;
begin
  n := n * 2;
  Twice := n;
  if n > 100 then exit;
  Twice := n + 1;
end;

begin
  x := 1;
  y := 2;
  Swap(x, y);
  writeln(x, ' ', y);
  writeln(Fact(10));
  writeln(IsEven(10), ' ', IsOdd(7));
  counter := 0;
  BumpTwice;
  Bump;
  writeln('counter = ', counter);
  Report(4);
  writeln(Twice(5), ' ', Twice(60));
end.
