program TempNames;
{ Temporaries that PascRust introduces mustn't clash with each other or
  with the program's own variables. }
type
   NodePtr = ^Node;
   Node = record
      value: integer;
      next: NodePtr;
   end;
var
   p, q, next: NodePtr;
   total: integer;

function Double(n: integer): integer;
begin
   Double := n * 2;
end;

begin
   new(p);
   new(q);
   p^.value := 1;
   q^.value := 100;
   total := Double(p^.value) + Double(q^.value);
   writeln(total);
   total := total + Double(p^.value);
   writeln(total);
   new(next);
   next^.value := 7;
   p^.next := nil;
   p := p^.next;
   writeln(p = nil, ' ', next^.value);
end.
