program DivisionByZero;
{ Dividing by a constant 0: TP says "Division by zero" when it compiles,
  with overflow checks on as off. }
const
   Z = 0;
   Bad = (10 div Z) + 1;
{ One error each, also where a constant is expected. }
type
   Sub = 1..(10 div Z);
   Name = string[8 div Z];
   Shape = record
      case Kind: Integer of
         1 div Z: (A: Integer);
   end;
var
   i: Integer;
   A: array[1..(4 div Z)] of Integer;
begin
   i := 7;
   WriteLn(i * (5 div 0));
   WriteLn(i + (5 mod Z));
   {$Q+}
   WriteLn(i * (5 div 0));
   case i of
      5 div Z: WriteLn('a');
   end;
end.
