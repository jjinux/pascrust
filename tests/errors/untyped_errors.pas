program UntypedErrors;
{ What untyped parameters can't do. }
var
  t: Text;

procedure Show(Data);
begin
end;

procedure Bad(var Data);
begin
  WriteLn(Data);
end;

procedure Dump(var Data);
begin
end;

begin
  Dump(t);
  Dump(5);
end.
