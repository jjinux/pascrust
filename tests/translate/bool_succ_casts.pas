{ Review batch L: Succ and Pred of a Boolean (the enum (False, True)),
  conversions of numbers (Word(-1) is 65535), not Odd(a) as a % 2 == 0,
  and shifts with arithmetic around them. Output from fpc -Mtp except
  Hi(i * 300): Free Pascal works in 32 bits (0), TP and Ruscal in 16. }
program BoolSuccAndCasts;
var
   i, a: Integer;
   ok: Boolean;
   c: Char;
   w: Word;
begin
   i := 5;
   a := 4;
   ok := False;
   c := 'C';
   WriteLn(Succ(ok), ' ', Pred(True), ' ', Ord(Succ(False)) * 3);
   WriteLn(Word(-1) div 2, ' ', Integer(-5) * 2, ' ', Byte(300), ' ', ShortInt(200));
   WriteLn(Char(66) < c, ' ', not Odd(a), ' ', not Odd(i) and ok);
   WriteLn(Succ(i) shl 1, ' ', Hi(i * 300), ' ', i shl a + 1, ' ', (i + a) shr 1);
   w := Word(-2);
   WriteLn(w);
end.
