program DivisionByZero;
{ Dividing by a constant 0: TP says "Division by zero" when it compiles,
  with overflow checks on as off. }
const
   Z = 0;
   Bad = (10 div Z) + 1;
var
   i: Integer;
begin
   i := 7;
   WriteLn(i * (5 div 0));
   WriteLn(i + (5 mod Z));
   {$Q+}
   WriteLn(i * (5 div 0));
end.
