program TypedConsts;
const
  Limit = 3;
  Greeting = 'Hi';
  Start: integer = 10;

procedure Count;
const
  calls: integer = 0;
begin
  calls := calls + 1;
  writeln('called ', calls, ' time(s)');
end;

begin
  writeln(Greeting, ' ', Limit, ' ', Start);
  Start := Start + Limit;
  writeln(Start);
  Count;
  Count;
end.
