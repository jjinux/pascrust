program AliasUnify;
{ Pause takes the global `generations` by var, and also calls a procedure
  that reads the global directly (the pattern from life.pas). Both names
  refer to the same variable, so the read must see the update. }
var
  generations: integer;

procedure Show;
begin
  writeln('generations = ', generations);
end;

procedure Pause(var gens: integer);
begin
  gens := gens + 5;
  Show;
  gens := gens + 1;
end;

begin
  generations := 1;
  Pause(generations);
  Show;
end.
