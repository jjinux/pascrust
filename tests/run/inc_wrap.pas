{ Integer arithmetic wraps, as in TP, even where rustc can see it will
  (it refuses that by default); and a negative step at the type's
  minimum. }
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
end.
