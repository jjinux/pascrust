{ A function without parameters kept in a variable, field or element: a
  value where TP wants one (`G := F`, `Run(F)`, `Assigned(F)`, `@F`), a
  call anywhere else (`N := R.Next`, `Fs[1] + Fs[2]`). }
program ProcValuesUncalled;
type
  IntFunc = function: Integer;
  Rec = record
    Next: IntFunc;
  end;
var
  F, G: IntFunc;
  R: Rec;
  Fs: array[1..2] of IntFunc;
  N, Counter: Integer;

function Tick: Integer;
begin
  Inc(Counter);
  Tick := Counter;
end;

function Ten: Integer;
begin
  Ten := 10;
end;

procedure Run(H: IntFunc);
begin
  WriteLn('run ', H);
end;

begin
  Counter := 0;
  F := Tick;
  G := F;
  Run(F);
  WriteLn(G, ' ', Counter);
  WriteLn(Assigned(F), ' ', @F = nil, ' ', @G = @Tick);
  R.Next := Tick;
  Fs[1] := Ten;
  Fs[2] := G;
  N := R.Next;
  WriteLn(N, ' ', Fs[1] + Fs[2], ' ', Counter);
  if R.Next = 6 then
    WriteLn('six')
  else
    WriteLn('not six');
  Run(Fs[1]);
  WriteLn(Assigned(R.Next), ' ', @Fs[2] = @Tick);
end.
