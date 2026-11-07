program LimitConstants;
{ A constant that is a type's limit has that type: High(Word) is a Word,
  High(Letter) of a Char subrange a Char. }
type
   Letter = 'a'..'z';
const
   TopB = High(Byte);
   TopW = High(Word);
   LowS = Low(ShortInt);
   TopC = High(Char);
   TopLet = High(Letter);
   LowLet = Low(Letter);
var
   C: Char;
   W: Word;
begin
   WriteLn(TopB, ' ', TopW, ' ', LowS, ' ', Ord(TopC));
   WriteLn(TopLet, LowLet);
   C := TopLet;
   if C = TopLet then
      WriteLn('last letter ', C);
   W := TopW;
   WriteLn(W, ' ', TopB + 1, ' ', LowS - 1);
   for C := LowLet to TopLet do
      Write(C);
   WriteLn;
end.
