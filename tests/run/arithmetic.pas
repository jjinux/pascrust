program Arithmetic;
{ Integer and real arithmetic, Turbo Pascal precedence, div/mod and
  16-bit wrap-around.
  NOTE: arithmetic.out is hand-edited: Free Pascal rounds round(-2.5) to -2
  (to even); Turbo Pascal rounds halves away from zero, giving -3. }
var
  a, b: integer;
  big: longint;
  small: byte;
  r: real;
begin
  a := 7;
  b := 2;
  writeln(a + b, ' ', a - b, ' ', a * b, ' ', a div b, ' ', a mod b);
  writeln(-7 div 2, ' ', -7 mod 2);
  writeln(a / b:0:2);
  writeln(2 + 3 * 4, ' ', (2 + 3) * 4);
  a := 32767;
  a := a + 1;
  writeln(a);
  big := 40000;
  big := big * 3;
  writeln(big);
  small := 200;
  writeln(small + 100);
  r := 1.5;
  writeln(r * a:0:1);
  writeln(sqr(5), ' ', abs(-3), ' ', sqrt(16.0):0:1);
  writeln(trunc(3.7), ' ', round(3.5), ' ', round(-2.5));
  writeln(odd(3), ' ', odd(4));
  writeln(maxint);
  writeln(7 shl 2, ' ', 100 shr 3, ' ', 6 and 3, ' ', 6 or 3, ' ', 6 xor 3);
end.
