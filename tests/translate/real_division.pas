{ Real division by zero stops with runtime error 200, as in TP (Rust's
  would give infinity). A divisor that's a constant other than 0 needs no
  check. }
program RealDivision;
const
   Two = 2.0;
var
   x, y: Real;
   i, j: Integer;
begin
   x := 3.0;
   y := 2.0;
   i := 7;
   j := 0;
   WriteLn(x / y:0:2, ' ', x / 2:0:2, ' ', x / Two:0:2, ' ', i / 2:0:2, ' ', -x / -4:0:2);
   y := 0;
   WriteLn(x / y:0:2);
   WriteLn('not reached');
end.
