{ x in [...] with constant elements is a match pattern; a constant
  expression in it (K + 1, Succ(Mon)) is written as its value. }
program SetPat;
type
   Day = (Mon, Tue, Wed, Thu, Fri, Sat, Sun);
const
   K = 5;
   First = 'a';
   Weekend = Sat;
var
   i: Integer;
   b: Byte;
   c: Char;
   d: Day;
begin
   i := 7;
   b := 6;
   c := 'c';
   d := Sun;
   WriteLn(i in [K + 1..K * 2], ' ', b in [K, K + 1], ' ', i in [-K..-1, 2 * K]);
   WriteLn(c in [First..Succ(Succ(First))], ' ', c in [Chr(Ord(First) + 1)]);
   WriteLn(d in [Succ(Mon), Weekend..Sun], ' ', d in [Pred(Sun)]);
   WriteLn(b in [K..2 * K], ' ', i in [K]);
end.
