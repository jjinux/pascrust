program ByteIndexes;
{ An index inside a place in a tagless variant part's bytes: an index
  reading another record is only read, and one reading the same record is
  worked out first (the place holds a guard on it). Output from
  fpc -Mtp. }
type
   Pun = record
      case Integer of
         0: (L: LongInt);
         1: (B: array[0..3] of Byte);
   end;
var
   p, q: Pun;
begin
   p.L := $01020304;
   q.L := 2;
   p.B[p.B[0] mod 4] := 7;
   WriteLn(p.L);
   p.B[q.B[0]] := 9;
   WriteLn(p.L, ' ', q.L);
end.
