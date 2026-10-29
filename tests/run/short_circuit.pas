program ShortCircuit;
{ The right side of and/or runs only when it has to, even when it reads
  through a pointer into a function's argument: (p <> nil) and
  Show(p^.value) mustn't read p^ when p is nil. Output from fpc -Mtp. }
type
  PNode = ^Node;
  Node = record
    value: Integer;
  end;
var
  p: PNode;

function Show(x: Integer): Integer;
begin
  WriteLn('Show ', x);
  Show := x;
end;

begin
  p := nil;
  if (p <> nil) and (Show(p^.value) > 0) then
    WriteLn('yes')
  else
    WriteLn('no');
  if (p = nil) or (Show(p^.value) > 0) then
    WriteLn('nil or positive');
  New(p);
  p^.value := 4;
  if (p <> nil) and (Show(p^.value) > 0) then
    WriteLn('yes');
  while (p <> nil) and (Show(p^.value) > 0) do
    p^.value := p^.value - 2;
  WriteLn('done');
end.
