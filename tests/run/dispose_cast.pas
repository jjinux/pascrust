program DisposeCast;
{ Freeing a record through a Pointer cast to its type,
  `Dispose(PNode(p))`, and a cast of a pointer to its own type. Output
  from fpc -Mtp. }
type
   PNode = ^Node;
   Node = record
      x: Integer;
   end;
var
   p: Pointer;
   n, m: PNode;
begin
   New(n);
   n^.x := 5;
   p := n;
   m := PNode(n);
   WriteLn(PNode(p)^.x, ' ', m^.x);
   Dispose(PNode(p));
   WriteLn('disposed');
end.
