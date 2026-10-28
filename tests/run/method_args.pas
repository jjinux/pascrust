program MethodArgs;
{ A method call whose arguments call methods of the same object: the
  arguments are computed before the object is borrowed for the call. }
type
  Counter = object
    n: integer;
    procedure Init;
    function Next: integer;
    function Twice(x: integer): integer;
    procedure Show(x: integer);
    procedure ShowNext;
  end;

procedure Counter.Init;
begin
  n := 0;
end;

function Counter.Next: integer;
begin
  n := n + 1;
  Next := n;
end;

function Counter.Twice(x: integer): integer;
begin
  n := n + 10;
  Twice := 2 * x;
end;

procedure Counter.Show(x: integer);
begin
  writeln('show ', x, ' (n = ', n, ')');
end;

procedure Counter.ShowNext;
begin
  Show(Next);
  Show(Twice(Next));
end;

var
  c: Counter;
begin
  c.Init;
  c.Show(c.Next);
  c.Show(c.Twice(5));
  c.ShowNext;
end.
