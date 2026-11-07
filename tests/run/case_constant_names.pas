{ A case label that names a constant keeps the name (ESC =>), so the
  Rust says where the value comes from; a Byte compared with Integer
  constants is widened for it. Expected output by hand: Free Pascal
  rejects the repeated label K, which TP allows (the first one wins). }
program Case1;
type
   Day = (Mon, Tue, Wed, Thu, Fri, Sat, Sun);
const
   K = 5;
   L = 9;
   Esc = #27;
   Enter = #13;
   Big = 1000;
   Last = Sun;
var
   i: Integer;
   b: Byte;
   c: Char;
   d: Day;
   n: LongInt;
begin
   i := 9;
   b := 5;
   c := #27;
   d := Sun;
   n := 1000;
   case i of
      1: WriteLn('one');
      K: WriteLn('K');
      6..L: WriteLn('6..L');
      K + 10: WriteLn('K + 10');
   end;
   case b of
      K: WriteLn('b K');
      7..L: WriteLn('b 7..L');
   end;
   case c of
      Esc: WriteLn('Esc');
      Enter, 'a'..'z': WriteLn('Enter or letter');
   end;
   case d of
      Mon..Fri: WriteLn('weekday');
      Last: WriteLn('last');
   end;
   case n of
      Big: WriteLn('Big');
      K: WriteLn('n K');
   end;
   case i of
      1..L: WriteLn('1..L');
      K: WriteLn('never');
   end;
end.
