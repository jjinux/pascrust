program Objects;
{ Turbo Pascal 5.5 objects: fields, methods, inheritance. }
const
  MaxSize = 5;
type
  Queue = object
    items: array[1..MaxSize] of integer;
    count: integer;
    procedure Create;
    procedure Add(item: integer);
    function Full: boolean;
    function Total: integer;
  end;

  NamedQueue = object(Queue)
    name: string[10];
    procedure Init(n: string);
    procedure Show;
  end;

var
  log: integer;   { a global that methods update }
  q: Queue;
  nq: NamedQueue;
  i: integer;

procedure Queue.Create;
begin
  count := 0;
end;

procedure Queue.Add(item: integer);
begin
  if not Full then
  begin
    count := count + 1;
    items[count] := item;
    log := log + 1;
  end;
end;

function Queue.Full: boolean;
begin
  Full := count = MaxSize;
end;

function Queue.Total: integer;
var
  sum: integer;

  procedure AddAll;
  var
    j: integer;
  begin
    { A procedure nested inside a method still sees the object's fields. }
    for j := 1 to count do
      sum := sum + items[j];
  end;

begin
  sum := 0;
  AddAll;
  Total := sum;
end;

procedure NamedQueue.Init(n: string);
begin
  Create;          { an inherited method }
  name := n;
end;

procedure NamedQueue.Show;
begin
  writeln(name, ': ', count, ' items, total ', Total);
end;

procedure Fill(var target: Queue; n: integer);
var
  k: integer;
begin
  for k := 1 to n do
    target.Add(k * 10);
end;

begin
  log := 0;
  q.Create;
  Fill(q, 3);
  writeln('q total ', q.Total, ' full ', q.Full);
  nq.Init('numbers');
  for i := 1 to 7 do
    nq.Add(i);
  nq.Show;
  writeln('full ', nq.Full, ' log ', log);
end.
