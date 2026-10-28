program IntegerTypes;
{ Negating a Byte or Word gives a signed result; a subrange too big for an
  Integer is stored in a LongInt; Ord and Abs work in constants. }
type
   Color = (Red, Green, Blue);
   Big = 0..70000;
const
   NColors = Ord(Blue) + 1;
   Dist = Abs(-7);
var
   b: byte;
   w: word;
   s: shortint;
   i: integer;
   l: longint;
   x: Big;
begin
   b := 5;
   w := 60000;
   s := -128;
   writeln(-b, ' ', -w, ' ', -s);
   i := -b;
   l := -w;
   writeln(i, ' ', l, ' ', -b * 1000);
   x := 65000;
   x := x + 1000;
   writeln(x);
   writeln(NColors, ' ', Dist);
end.
