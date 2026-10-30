{ A routine kept in a unit's procedural type that uses a global of a type
  the program declares: the unit names the program's type. }
program ProcUnitTypes;
uses ProcUnit;
type
  Stats = record
    Count, Total: Integer;
  end;
var
  S: Stats;
procedure Tally(N: Integer);
begin
  Inc(S.Count);
  S.Total := S.Total + N;
end;
begin
  S.Count := 0;
  S.Total := 0;
  OnEvent := Tally;
  Fire(3);
  Fire(4);
  WriteLn(S.Count, ' ', S.Total);
end.
