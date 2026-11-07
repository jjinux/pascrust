{ An Inc or Dec step that names a constant keeps the name; a step that
  doesn't fit the target is cut down with `as`, which keeps the low bits
  as TP does (300 is 44 in a Byte). }
program IncNamedSteps;
const
   K = 5;
   Big = 300;
   Neg = -3;
   Huge = 40000;
var
   b: Byte;
   i: Integer;
   c: Char;
begin
   b := 10;
   i := 10;
   c := 'a';
   Inc(b, Big);
   Dec(b, Neg);
   Inc(b, K);
   Dec(i, Neg);
   Inc(i, Huge);
   Inc(c, K);
   WriteLn(b, ' ', i, ' ', c);
   {$Q+}
   b := 10;
   i := 10;
   Dec(b, Neg);
   Inc(b, Big);
   Dec(i, Neg);
   Inc(b, Neg);
   Dec(b, -K);
   WriteLn(b, ' ', i);
end.
