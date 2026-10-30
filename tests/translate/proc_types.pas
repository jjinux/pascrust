program ProcTypes;
{ Procedural types: variables, parameters and a typed constant of them;
  calls through them, also with var parameters, through a record field
  and an array element; a parameterless function value called in an
  expression; nil, Assigned and `@` comparisons. Output from fpc -Mtp,
  except SizeOf(Op): a procedural value is TP's 4-byte far address, where
  Free Pascal's is 8. }
type
   SwapProc = procedure(var X, Y: Integer);
   IntFunc = function: Integer;
   Op = function(A, B: Integer): Integer;
   Entry = record
      Name: string[5];
      Apply: Op;
   end;
var
   S: SwapProc;
   Next: IntFunc;
   Counter, A, B, i: Integer;
   Ops: array[1..3] of Op;
   E: Entry;
   N: Integer;
procedure Swap(var X, Y: Integer);
   var T: Integer;
   begin
      T := X; X := Y; Y := T;
   end;
function Tick: Integer;
   begin
      Inc(Counter);
      Tick := Counter;
   end;
function Add(A, B: Integer): Integer; begin Add := A + B; end;
function Mul(A, B: Integer): Integer; begin Mul := A * B; end;
function Sub(A, B: Integer): Integer; begin Sub := A - B; end;
function Fold(F: Op; X, Y: Integer): Integer;
   begin
      Fold := F(F(X, Y), Y);
   end;
const
   Default: Op = Add;
begin
   A := 1; B := 2;
   S := Swap;
   S(A, B);
   WriteLn(A, ' ', B);
   Counter := 0;
   Next := Tick;
   N := Next;
   N := N + Next;
   WriteLn(N, ' ', Counter);
   Ops[1] := Add; Ops[2] := Mul; Ops[3] := Sub;
   for i := 1 to 3 do
      Write(Ops[i](7, 3), ' ');
   WriteLn;
   E.Name := 'mul';
   E.Apply := Mul;
   WriteLn(E.Name, ' ', E.Apply(6, 7), ' ', Fold(E.Apply, 2, 3), ' ', Fold(Default, 2, 3));
   Ops[2] := nil;
   WriteLn(Assigned(Ops[1]), ' ', Assigned(Ops[2]));
   WriteLn(@E.Apply = @Mul, ' ', @E.Apply = @Add, ' ', @Ops[2] = nil, ' ', @Ops[1] <> nil, ' ', SizeOf(Op));
end.
