{ Review batch M: more conversions of numbers (out of range, nested, as
  the receiver of a method), not Odd of expressions, and Succ of a
  comparison. Output from fpc -Mtp, except Integer(-1) shr 1: TP shifts
  in 16 bits (32767), Free Pascal in 64. }
program CastsFolded;
var
   i, a: Integer;
   l: LongInt;
   b: Byte;
begin
   i := 5;
   a := 2;
   l := -3;
   b := 7;
   WriteLn(ShortInt(128), ' ', Integer(32768), ' ', Word(70000), ' ', Ord(Char(256)), ' ', Ord(Char(-1)));
   WriteLn(Word(Integer(-1)), ' ', Integer(Word(65535)), ' ', LongInt(Word(-1)), ' ', Word(+5), ' ', Integer(+5));
   WriteLn(Abs(Integer(-5)), ' ', Sqr(Integer(-5)), ' ', Lo(Integer(-300)), ' ', Integer(-1) shl a, ' ', Integer(-1) shr 1);
   WriteLn(not Odd(i + 1), ' ', not Odd(-i), ' ', not not Odd(i), ' ', not Odd(l), ' ', not Odd(b));
   WriteLn(Succ(i < 3), ' ', Pred(i = 5), ' ', i + 1 shl 1, ' ', i shl 1 shl 2, ' ', Word(Integer(-1) shr 1));
end.
