program ShiftWide;
{ Shifting by the integer's width or more shifts every bit out. (Free Pascal
  computes in 64 bits here, so this test's .out is written by hand from
  Turbo Pascal's 16- and 32-bit results.) }
var
   i: integer;
   w: word;
   l: longint;
   n: integer;
begin
   i := 1;
   w := 65535;
   l := 1;
   n := 20;
   writeln(i shl 20, ' ', i shl n, ' ', i shl 4, ' ', w shr n, ' ', w shr 16);
   i := -1;
   writeln(i shr n, ' ', i shr 15, ' ', l shl n, ' ', l shl 32);
   n := 3;
   writeln(i shl n, ' ', w shr n);
end.
