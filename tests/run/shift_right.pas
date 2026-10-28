program ShiftRight;
{ shr is a logical shift: it fills with zeros, even for negative numbers.
  (Free Pascal computes in 64 bits here, so this test's .out is written by
  hand from Turbo Pascal's 16- and 32-bit results.) }
var
   i: integer;
   l: longint;
   s: shortint;
   w: word;
begin
   i := -300;
   l := -300;
   s := -3;
   w := 65236;
   writeln(i shr 4, ' ', l shr 4, ' ', s shr 1, ' ', -300 shr 4);
   writeln(300 shr 4, ' ', w shr 4);
end.
