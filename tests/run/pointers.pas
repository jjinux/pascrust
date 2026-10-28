program Pointers;
{ Pointers and the heap: a linked list, and the binary search tree
  algorithm from examples/database/bstunit.pas. }
type
  NodePtr = ^Node;
  Node = record
    value: integer;
    next: NodePtr;
  end;

  KeyPtr = ^Key;
  Key = record
    id: longint;
    before, after: KeyPtr;
  end;

var
  head, p: NodePtr;
  i: integer;
  root: KeyPtr;
  ok: boolean;
  k: Key;

procedure Push(var list: NodePtr; v: integer);
var
  n: NodePtr;
begin
  new(n);
  n^.value := v;
  n^.next := list;
  list := n;
end;

function Sum(list: NodePtr): integer;
var
  total: integer;
begin
  total := 0;
  while list <> nil do
  begin
    total := total + list^.value;
    list := list^.next;
  end;
  Sum := total;
end;

procedure Insert(var root: KeyPtr; id: longint; var successful: boolean);
begin
  if root = nil then
  begin
    new(root);
    root^.id := id;
    root^.before := nil;
    root^.after := nil;
  end
  else if id < root^.id then
    Insert(root^.before, id, successful)
  else if id > root^.id then
    Insert(root^.after, id, successful)
  else
    successful := false;
end;

procedure Delete(var root: KeyPtr; item: Key; var successful: boolean);
var
  temp: KeyPtr;
begin
  if root = nil then
    successful := false
  else if item.id < root^.id then
    Delete(root^.before, item, successful)
  else if item.id > root^.id then
    Delete(root^.after, item, successful)
  else if root^.before = nil then
  begin
    temp := root^.after;
    dispose(root);
    root := temp;
  end
  else if root^.after = nil then
  begin
    temp := root^.before;
    dispose(root);
    root := temp;
  end
  else
  begin
    temp := root^.after;
    while temp^.before <> nil do
      temp := temp^.before;
    root^ := temp^;
    Delete(root^.after, temp^, successful);
  end;
end;

procedure Traverse(root: KeyPtr);
begin
  if root <> nil then
  begin
    Traverse(root^.before);
    write(root^.id, ' ');
    Traverse(root^.after);
  end;
end;

begin
  head := nil;
  for i := 1 to 5 do
    Push(head, i * i);
  writeln('sum = ', Sum(head));
  p := head;
  p := p^.next;
  writeln('second = ', p^.value);

  root := nil;
  ok := true;
  Insert(root, 50, ok);
  Insert(root, 30, ok);
  Insert(root, 70, ok);
  Insert(root, 20, ok);
  Insert(root, 40, ok);
  Insert(root, 60, ok);
  Insert(root, 80, ok);
  Insert(root, 30, ok);
  writeln('duplicate accepted: ', ok);
  Traverse(root);
  writeln;
  k.id := 50;
  ok := true;
  Delete(root, k, ok);
  k.id := 20;
  Delete(root, k, ok);
  Traverse(root);
  writeln;
  writeln('root is now ', root^.id, ', deleted ok: ', ok);
end.
