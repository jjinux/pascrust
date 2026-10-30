program VariantNested;
{ Variant parts nested in a variant: tagged inside tagged (flattened; a
  file keeps the variants both tags select), a tagged part inside a
  tagless one (all of it TP's bytes, so the inner fields pun with the
  outer ones), and an anonymous record in a variant. Output from
  fpc -Mtp. }
type
   Shape = record
      Id: Integer;
      case Kind: Integer of
         0: (Radius: Integer);
         1: (Width: Integer;
             case Filled: Integer of
               0: (Outline: Integer);
               1: (Color, Pattern: Integer));
   end;
   Pun = record
      case Integer of
         0: (L: LongInt);
         1: (Lo: Word;
             case Integer of
               0: (Hi: Word);
               1: (B3, B4: Byte));
         2: (Pair: record
               A, B: Word;
             end);
   end;
var
   s, t: Shape;
   f: file of Shape;
   p: Pun;
begin
   s.Id := 7;
   s.Kind := 1;
   s.Width := 10;
   s.Filled := 1;
   s.Color := 4;
   s.Pattern := 2;
   WriteLn(SizeOf(Shape), ' ', s.Width * s.Color + s.Pattern);
   Assign(f, 'variant_nested.dat');
   Rewrite(f);
   Write(f, s);
   s.Kind := 0;
   s.Radius := 5;
   Write(f, s);
   Close(f);
   Reset(f);
   Read(f, t);
   WriteLn(t.Id, ' ', t.Kind, ' ', t.Width, ' ', t.Filled, ' ', t.Color, ' ', t.Pattern);
   Read(f, t);
   WriteLn(t.Kind, ' ', t.Radius, ' ', FileSize(f));
   Close(f);
   Erase(f);
   p.L := $1ABBCCDD;
   WriteLn(SizeOf(Pun), ' ', p.Lo, ' ', p.Hi, ' ', p.B3, ' ', p.B4);
   p.B4 := 0;
   WriteLn(p.L, ' ', p.Pair.A, ' ', p.Pair.B);
   p.Pair.B := 1;
   WriteLn(p.L);
end.
