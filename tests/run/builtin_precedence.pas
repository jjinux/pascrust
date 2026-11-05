{ Standard functions whose Rust is an expression, not a call, inside a
  larger one: Succ(i) * 2 is (i + 1) * 2, Odd(a) = Odd(b) compares two
  comparisons, a cast before < gets parentheses. Output from fpc -Mtp
  except Hi(i * 300) * 2: Free Pascal works out i * 300 in 32 bits
  (high byte 0), TP and PascRust in 16 (5). }
program BuiltinPrecedence;
var
   i, j: Integer;
   c: Char;
   b: Byte;
begin
   i := 5;
   j := 3;
   c := 'A';
   b := 200;
   WriteLn(Succ(i) * 2, ' ', Pred(i) * 2, ' ', Odd(i) = Odd(j));
   WriteLn(Ord(c) < 70, ' ', Ord(c) * 2, ' ', Abs(-i) * 2, ' ', Sqr(i) * 2);
   WriteLn(Chr(Ord(c) + 1) < 'C', ' ', Hi(i * 300) * 2, ' ', Lo(i) * 2, ' ', Swap(i) * 2);
   WriteLn(Round(2.4) * 2, ' ', Trunc(2.7) * 2, ' ', Length('abc') * 2, ' ', Ord(b) * 2);
end.
