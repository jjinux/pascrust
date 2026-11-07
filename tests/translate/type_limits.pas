{ High and Low of a type are names in the Rust, where its type is known:
  i16::MAX for High(Integer), N for High(Index) with Index = 1..N. }
program TypeLimits;
const
   N = 10;
   Top = High(Integer);
   First = 'a';
   Last = 'z';
type
   Day = (Mon, Tue, Wed, Thu, Fri, Sat, Sun);
   Index = 1..N;
   Letter = First..Last;
   Weekday = Mon..Fri;
var
   i: Integer;
   w: Word;
   l: LongInt;
   b: Byte;
   C: Char;
   s: ShortInt;
   x: Index;
begin
   i := High(Integer);
   l := High(Integer);
   w := High(Word);
   l := Low(LongInt);
   b := High(b);
   C := High(Char);
   s := Low(ShortInt);
   x := High(Index);
   i := Low(Index);
   if i < High(i) then
      WriteLn(i, ' ', l, ' ', w, ' ', b, ' ', Ord(C), ' ', s, ' ', x);
   WriteLn(High(Integer), ' ', High(Integer) + 1, ' ', High(Index));
   { A Char or enum subrange has its own bounds. }
   WriteLn(Low(Letter), High(Letter), ' ', Ord(High(Weekday)), ' ', Top);
   for x := Low(Index) to High(Index) do
      Write(x);
   WriteLn;
   case i of
      Low(Integer): WriteLn('low');
      High(Integer): WriteLn('high');
      else
         WriteLn('in between');
   end;
end.
