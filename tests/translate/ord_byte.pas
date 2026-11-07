{ Ord of a Char or a Byte, used as a Byte, is the same byte (b = c),
  with overflow checks too. }
program OrdByte;
var
   b, b2: Byte;
   c: Char;
   i: Integer;
begin
   c := 'C';
   b2 := 7;
   b := Ord(c);
   WriteLn(b);
   b := Ord('A');
   WriteLn(b);
   b := Ord(b2);
   i := Ord(c);
   {$Q+}
   Inc(b, Ord('A'));
   Inc(b, Ord(c));
   WriteLn(b, ' ', i);
end.
