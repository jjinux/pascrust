program ArgOrder;
{ Arguments run left to right, even when a later one is computed into a
  temporary first (it reads through a pointer): Bump changes the record
  before head^.value is read. Output from fpc -Mtp. }
type
  PNode = ^Node;
  Node = record
    value: Integer;
  end;
var
  head: PNode;

function Bump: Integer;
begin
  head^.value := head^.value + 10;
  Bump := 1;
end;

procedure Show(a: Integer; var b: Integer);
begin
  WriteLn(a, ' ', b);
end;

procedure ShowV(a, b: Integer);
begin
  WriteLn(a, ' ', b);
end;

begin
  New(head);
  head^.value := 5;
  Show(Bump, head^.value);
  head^.value := 5;
  ShowV(Bump, head^.value);
end.
