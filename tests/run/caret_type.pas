program CaretType;
{ ^T is a pointer to T where a type can appear, and Ctrl-T elsewhere. }
type
  T = record
    value: Integer;
  end;
const
  Tab = ^I;
var
  p: ^T;
  c: Char;
begin
  New(p);
  p^.value := 42;
  c := ^T;
  WriteLn(p^.value, ' ', Ord(c), ' ', Ord(Tab));
  Dispose(p);
end.
