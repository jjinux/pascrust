{ Procedural fields, elements and pointers called as statements (`H.S;`,
  `with H do S;`, `Arr[1];`, `PS^;`), a parameterless function value in a
  standard function's argument (`Abs(F)`: called), and an object's field
  holding one, used in its methods. Output from fpc -Mtp, except lines 6
  and 7, which it refuses (it doesn't call `F` in `Odd(F)`). }
program ProcFieldCalls;
type
   Action = procedure;
   IntFunc = function: Integer;
   PAction = ^Action;
   Holder = record
      S: Action;
      Next: IntFunc;
   end;
   Counter = object
      Next: IntFunc;
      procedure Step;
      function Twice: Integer;
   end;
var
   H: Holder;
   P: ^Holder;
   Arr: array[1..2] of Action;
   PS: PAction;
   F: IntFunc;
   C: Counter;
   N, Calls: Integer;

procedure Hello;
   begin
      Inc(Calls);
      WriteLn('hello ', Calls);
   end;

function Tick: Integer;
   begin
      Inc(N);
      Tick := N;
   end;

procedure Counter.Step;
   var
      K: Integer;
   begin
      K := Next;
      WriteLn(K, ' ', Assigned(Next));
   end;

function Counter.Twice: Integer;
   begin
      Twice := Next + Next;
   end;

begin
   Calls := 0;
   N := 0;
   H.S := Hello;
   H.S;
   with H do
      S;
   New(P);
   P^.S := Hello;
   P^.S;
   Arr[1] := Hello;
   Arr[1];
   New(PS);
   PS^ := Hello;
   PS^;
   F := Tick;
   WriteLn(Odd(F), ' ', Abs(F), ' ', Sqr(F));
   H.Next := Tick;
   WriteLn(Abs(H.Next), ' ', Assigned(H.Next));
   C.Next := Tick;
   C.Step;
   WriteLn(C.Twice);
end.
