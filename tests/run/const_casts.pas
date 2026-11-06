{ Typecasts in constant expressions, which TP 7 allows: Word(-1) is 65535,
  as a constant, a case label, an array bound and a set element. Output
  from fpc -Mtp. }
program ConstCasts;
type
   Color = (Red, Green, Blue);
const
   W1 = Word(-1);
   B1 = Byte(300);
   S1 = ShortInt(200);
   C1 = Char(65);
   Last = Color(2);
   Flag = Boolean(1);
   Half = Word(-1) div 2;
   Code = Ord(Char(66)) + 1;
var
   b: Byte;
   a: array[0..Byte(258)] of Integer;
   i: Integer;
begin
   WriteLn(W1, ' ', B1, ' ', S1, ' ', C1, ' ', Ord(Last), ' ', Flag, ' ', Half, ' ', Code);
   WriteLn(W1 - 1, ' ', High(a), ' ', C1 in [Char(64)..Char(70)]);
   for i := 0 to 2 do
      a[i] := i * 10;
   WriteLn(a[Byte(258)]);
   b := 44;
   case b of
      Byte(300):
         WriteLn('forty-four');
      else
         WriteLn('other');
   end;
end.
