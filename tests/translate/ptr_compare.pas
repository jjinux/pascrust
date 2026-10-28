program PtrCompare;
{ Comparing pointers compares what they point at, not the records. And a
  path through two pointers changes only the last record. }
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
   p^.next^.value := 5;
   writeln(p^.value, ' ', q^.value);
   q := p;
   writeln(p = q);
end.
