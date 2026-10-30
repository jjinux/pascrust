program VariantPunning;
{ Variant parts without a tag field, the type-punning idiom: the variants
  share TP's bytes, so a LongInt reads as two Words or four Bytes, and a
  string[3] over the same bytes sees its length byte. In-place changes
  (Inc, an element, a var argument), Move into one, an untyped parameter,
  a typed constant, and a file
  of them. Output from fpc -Mtp, except SizeOf(Pun): Free
  Pascal pads the variants after the 1-byte Tag, so it says 8 where TP
  says 5. }
type
   Pun = record
      Tag: Char;
      case Integer of
         0: (L: LongInt);
         1: (Lo, Hi: Word);
         2: (B: array[1..4] of Byte);
         3: (S: string[3]);
   end;
const
   Start: Pun = (Tag: 's'; L: 65539);
var
   p, q: Pun;
   n: LongInt;
   f: file of Pun;
   i: Integer;

procedure Bump(var w: Word);
   begin
      Inc(w, 2);
   end;

function Sum(var data; size: Integer): Integer;
   type
      TBytes = array[1..100] of Byte;
   var
      k, total: Integer;
   begin
      total := 0;
      for k := 1 to size do
         total := total + TBytes(data)[k];
      Sum := total;
   end;

begin
   p.Tag := 'x';
   p.L := $12345678;
   WriteLn(p.Lo, ' ', p.Hi, ' ', p.B[1], ' ', p.B[4], ' ', SizeOf(Pun));
   Inc(p.Hi);
   WriteLn(p.L);
   p.B[1] := 0;
   WriteLn(p.L, ' ', p.Lo);
   Bump(p.Lo);
   WriteLn(p.Lo);
   p.S := 'abc';
   WriteLn(p.B[1], ' ', p.B[2], ' ', p.L);
   with p do
      WriteLn(S, ' ', Length(S));
   n := 1000000;
   Move(n, q.L, SizeOf(n));
   WriteLn(q.Lo, ' ', q.Hi);
   WriteLn(Sum(p, SizeOf(Pun)));
   q := p;
   WriteLn(q.S, ' ', q.Tag);
   Assign(f, 'variant_punning.dat');
   Rewrite(f);
   for i := 1 to 3 do
   begin
      p.L := i * 65537;
      Write(f, p);
   end;
   Close(f);
   Reset(f);
   Seek(f, 1);
   Read(f, q);
   WriteLn(q.Lo, ' ', q.Hi, ' ', FileSize(f));
   Close(f);
   Erase(f);
   WriteLn(Start.Tag, ' ', Start.Lo, ' ', Start.Hi);
end.
