program WithStmt;
{ with r do ... lets the body use r's fields (and an object's methods) by
  their bare names. }
type
  Point = record
    x, y: Integer;
  end;
  Rect = record
    topLeft, size: Point;
    name: string[10];
  end;
  PNode = ^Node;
  Node = record
    value: Integer;
    next: PNode;
  end;
  Counter = object
    count: Integer;
    procedure Bump;
  end;

var
  r: Rect;
  pts: array[1..3] of Point;
  i, x: Integer;
  head, p: PNode;
  c: Counter;

procedure Counter.Bump;
begin
  count := count + 1;
end;

begin
  x := 100;
  with r do
  begin
    name := 'box';
    with topLeft do
    begin
      x := 1;  { the field: the innermost with wins }
      y := 2;
    end;
    with size do
      x := 30;
    size.y := 40;
  end;
  WriteLn(r.name, ' ', r.topLeft.x, ' ', r.topLeft.y, ' ', r.size.x, ' ', r.size.y, ' ', x);
  with r, topLeft do
    WriteLn(name, ' ', x + y);
  for i := 1 to 3 do
    with pts[i] do
    begin
      x := i;
      y := i * i;
    end;
  WriteLn(pts[3].x, ' ', pts[3].y);
  { A list built with with p^ do: the body changes p, but the with still
    means the record p pointed to when it started. }
  head := nil;
  for i := 1 to 3 do
  begin
    New(p);
    with p^ do
    begin
      value := i * 10;
      next := head;
      head := p;
      p := nil;
      value := value + 1;
    end;
  end;
  p := head;
  while p <> nil do
    with p^ do
    begin
      Write(value, ' ');
      p := next;
    end;
  WriteLn;
  with head^.next^ do
    WriteLn(value);
  c.count := 0;
  with c do
  begin
    Bump;
    Bump;
    WriteLn(count);
  end;
end.
