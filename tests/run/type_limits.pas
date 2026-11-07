{ High and Low of a type are names in the Rust, where its type is known:
  i16::MAX for High(Integer), N for High(Index) with Index = 1..N. }
program TypeLimits;
const
   N = 10;
type
   Index = 1..N;
var
   i: Integer;
   w: Word;
   l: LongInt;
   b: Byte;
   c: Char;
   s: ShortInt;
   x: Index;
begin
   i := High(Integer);
   l := High(Integer);
   w := High(Word);
   l := Low(LongInt);
   b := High(b);
   c := High(Char);
   s := Low(ShortInt);
   x := High(Index);
   i := Low(Index);
   if i < High(i) then
      WriteLn(i, ' ', l, ' ', w, ' ', b, ' ', Ord(c), ' ', s, ' ', x);
   WriteLn(High(Integer), ' ', High(Integer) + 1, ' ', High(Index));
end.
