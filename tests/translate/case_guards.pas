{ A case label worked out from a constant (K + 1) can't be a Rust
  pattern: its arm tests it in a guard, keeping the name. }
program CaseGuards;
const
   K = 5;
   L = 9;
var
   i: Integer;
   n: LongInt;
   c: Char;
begin
   for i := 1 to 16 do
      case i of
         1, K + 1: Write('a');
         K * 2..K * 3: Write('b');
         L: Write('c');
         K + 2..L - 1: Write('d');
         else
            Write('.');
      end;
   WriteLn;
   n := 5;
   case n of
      K: WriteLn('n K');
      L + 1: WriteLn('n L + 1');
   end;
   c := 'f';
   case c of
      Chr(Ord('a') + K): WriteLn('f');
   end;
end.
