{ Constants worked out from other constants keep the names in the Rust
  (const M = K * 2 + 1 is `K * 2 + 1`), and are worked out in 32 bits, as
  TP does: C + 1 with C = 32767 is 32768, even where C is an Integer. }
program ConstantExpressions;
const
   K = 5;
   C = 32767;
   M = K * 2 + 1;
   Lo = 1;
   Hi = Lo + K;
   Z = $FF and K;
   ChK = Chr(Ord('A') + K);
   Wide = C + 1;
   Pi2 = 3.14 * 2;
   Neg = -K;
var
   i: Integer;
   l: LongInt;
begin
   i := 2;
   l := K + 30000;
   WriteLn(l);
   i := i * (30000 + 30000 - 40000);
   WriteLn(i);
   i := 3;
   { C + 1 is a LongInt constant, so the arithmetic around it is too. }
   WriteLn(i * (C + 1), ' ', (C + 1) div i);
   if i < C + 1 then
      WriteLn('less');
   i := i * (C + 1);
   WriteLn(i);
   l := C + 1;
   WriteLn(l, ' ', C + 1);
   WriteLn(30000 + 30000, ' ', (30000 + 30000) div 2);
   WriteLn(M, ' ', Hi, ' ', Z, ' ', ChK, ' ', Wide, ' ', Pi2:0:2, ' ', Neg);
end.
