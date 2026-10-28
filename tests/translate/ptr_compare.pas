program PtrCompare;
{ Comparing pointers compares what they point at, not the records. }
type
   NodePtr = ^Node;
   Node = record
      value: integer;
      next: NodePtr;
   end;
var
   p, q: NodePtr;
begin
   new(p);
   new(q);
   p^.value := 1;
   q^.value := 1;
   p^.next := q;
   writeln(p = q, ' ', p <> q, ' ', p^.next = q, ' ', q^.next = nil);
   q := p;
   writeln(p = q);
end.
