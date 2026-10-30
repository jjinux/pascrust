{ New and Dispose through a Pointer cast to a typed pointer: the record
  is kept in the Pointer, and read back through the same cast. A cast
  to the pointer's own type changes nothing. }
program NewThroughPointer;
type
  PNode = ^TNode;
  TNode = record
    v: Integer;
    name: string[10];
  end;
var
  q: Pointer;
  n: PNode;
begin
  New(PNode(q));
  PNode(q)^.v := 5;
  PNode(q)^.name := 'five';
  n := q;
  WriteLn(n^.v, ' ', PNode(q)^.name);
  Inc(n^.v);
  WriteLn(PNode(q)^.v);
  Dispose(PNode(q));
  New(PNode(n));
  n^.v := 3;
  WriteLn(n^.v);
end.
