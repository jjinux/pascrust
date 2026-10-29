program EvalOrder;
{ When a function in an expression has side effects: Write(F, G) writes
  F's value before G runs (TP defines it as Write(F); Write(G)), and
  n := n + Bump reads n before Bump changes it, as Free Pascal does. }
var
  n: Integer;

function Bump: Integer;
begin
  n := n + 10;
  Bump := 1;
end;

function F: Integer;
begin
  Write('F');
  F := 1;
end;

function G: Integer;
begin
  Write('G');
  G := 2;
end;

begin
  n := 0;
  n := n + Bump;
  WriteLn('n + Bump: ', n);
  n := 0;
  n := Bump + n;
  WriteLn('Bump + n: ', n);
  WriteLn(F, G, ' Write(F, G)');
  Write(F, ' ');
  WriteLn(G);
end.
