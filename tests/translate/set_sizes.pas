program SetSizes;
{ A set takes the bytes TP gave it, from its lowest value's byte to its
  highest's: set of 0..7 is 1 byte, set of 'a'..'z' 4 (bytes 12 to 15 of
  the whole bitmap), in SizeOf, untyped arguments, arrays and files.
  Output from fpc -Mtp, except the file's bytes: Free Pascal pads the
  record (9 130 0 0 4 0 0 2), where TP doesn't. }
type
   Bits = set of 0..7;
   Letters = set of 'a'..'z';
   Rec = record
      n: Byte;
      b: Bits;
      l: Letters;
   end;
   TBytes = array[0..255] of Byte;
var
   r, r2: Rec;
   arr: array[1..2] of Bits;
   f: file of Rec;
   g: file of Byte;
   x: Byte;
procedure Dump(var data; n: Integer);
   var i: Integer;
   begin
      for i := 0 to n - 1 do Write(TBytes(data)[i], ' ');
      WriteLn;
   end;
begin
   WriteLn(SizeOf(Bits), ' ', SizeOf(Letters), ' ', SizeOf(arr));
   r.n := 9; r.b := [1, 7]; r.l := ['b', 'y'];
   Dump(r.b, SizeOf(r.b));
   Dump(r.l, SizeOf(r.l));
   arr[1] := [0, 1]; arr[2] := [7];
   Dump(arr, SizeOf(arr));
   Assign(f, 'set_sizes.dat'); Rewrite(f); Write(f, r); Close(f);
   Assign(g, 'set_sizes.dat'); Reset(g);
   while not Eof(g) do begin Read(g, x); Write(x, ' '); end;
   WriteLn; Close(g);
   Reset(f); Read(f, r2); Close(f); Erase(f);
   WriteLn(7 in r2.b, ' ', 2 in r2.b, ' ', 'y' in r2.l, ' ', 'c' in r2.l);
end.
