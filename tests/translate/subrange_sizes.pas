program SubrangeSizes;
{ A subrange takes the smallest integer type that holds its bounds, as in
  TP: 1..12 is a byte, -5..5 a ShortInt, 0..1000 an Integer, 0..65535 a
  Word. That's what SizeOf says and what a record in a file holds; and
  arithmetic on them still works in Integer. Output from fpc -Mtp. }
type
   TMonth = 1..12;
   Date = record
      Month: TMonth;
      Day: 1..31;
      Offset: -5..5;
      Spare: 0..255;
      Year: 0..9999;
      Count: 0..65535;
   end;
var
   d: Date;
   f: file of Date;
   b: file of Byte;
   x: Byte;
   m: TMonth;
   total: Integer;
begin
   WriteLn(SizeOf(TMonth), ' ', SizeOf(Date));
   d.Month := 12;
   d.Day := 31;
   d.Offset := -5;
   d.Spare := 255;
   d.Year := 1994;
   d.Count := 65535;
   total := 0;
   for m := 1 to 12 do
      total := total + m * d.Day;
   WriteLn(total, ' ', d.Month * d.Day, ' ', d.Offset - 1, ' ', d.Count);
   Assign(f, 'subrange_sizes.dat');
   Rewrite(f);
   Write(f, d);
   Close(f);
   Assign(b, 'subrange_sizes.dat');
   Reset(b);
   while not Eof(b) do
   begin
      Read(b, x);
      Write(x, ' ');
   end;
   WriteLn;
   Close(b);
   Erase(b);
end.
