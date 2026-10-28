program PointerScope;
{ ^Node means the Node declared in the same type section, even when an
  outer scope has a Node too. }
type
   Node = record
      a: integer;
   end;

procedure Local;
type
   NodePtr = ^Node;
   Node = record
      b: integer;
   end;
var
   p: NodePtr;
begin
   new(p);
   p^.b := 1;
   writeln(p^.b);
   dispose(p);
end;

var
   n: Node;
begin
   n.a := 2;
   Local;
   writeln(n.a);
end.
