program PtrArgs;
{ A value argument read through a pointer (p^ or p^.field) is copied
  before the call, so the callee may change or dispose the record. }
type
  NodePtr = ^Node;
  Node = record
    value: integer;
    next: NodePtr;
  end;

var
  head: NodePtr;

procedure FreeAndShow(n: Node; count: integer);
begin
  dispose(head);
  head := nil;
  writeln('freed node ', n.value, ' (count ', count, ')');
end;

procedure Bump(var p: NodePtr; old: integer);
begin
  p^.value := old + 1;
end;

begin
  new(head);
  head^.value := 41;
  head^.next := nil;
  Bump(head, head^.value);
  writeln('value is now ', head^.value);
  FreeAndShow(head^, head^.value);
  writeln('head is nil: ', head = nil);
end.
