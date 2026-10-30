program TaglessRecords;
{ A record (itself with a tagless variant part) and an enum as fields of
  a tagless variant part: kept in its bytes like any other field, and a
  nested field set from its own value. Output from fpc -Mtp. }
type
   Inner = record
      case Integer of
         0: (W: Word);
         1: (Lo, Hi: Byte);
   end;
   Color = (Red, Green, Blue);
   Pun = record
      case Integer of
         0: (L: LongInt);
         1: (I1, I2: Inner);
         2: (C: Color);
   end;
var
   p: Pun;
begin
   p.L := $01020304;
   WriteLn(p.I1.W, ' ', p.I2.Hi);
   p.I2.Hi := p.I2.Hi + 1;
   WriteLn(p.L);
   p.C := Blue;
   WriteLn(Ord(p.C));
end.
