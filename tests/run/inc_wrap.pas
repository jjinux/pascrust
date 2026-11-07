{ Integer arithmetic wraps, as in TP, even where rustc can see it will
  (it refuses that by default); and a negative step at the type's
  minimum, or one just past the type's top (128 in a ShortInt
  adds -128). }
program IncWrap;
var
   i: Integer;
   sb: ShortInt;
   b: Byte;
begin
   i := 0;
   Inc(i, -32768);
   sb := 0;
   Dec(sb, -128);
   WriteLn(i, ' ', sb);
   b := 250;
   Inc(b, 10);
   WriteLn(b);
   b := 5;
   Dec(b, 10);
   WriteLn(b);
   b := 10;
   Inc(b, -200);
   WriteLn(b);
   b := 250;
   b := b + 10;
   WriteLn(b);
   Inc(b, 256);
   WriteLn(b);
   sb := 10;
   Inc(sb, 128);
   i := 10;
   Inc(i, 32768);
   WriteLn(sb, ' ', i);
   Dec(i, 32768);
   WriteLn(i);
end.
