program UntypedOffsets;
{ Untyped arguments that start inside a variable: an element at a
  variable index (of a byte array and a Word array), a field after a
  variant part (all variants start at the same byte), and Move from one.
  An index out of range is runtime error 201 (TP read memory before the
  array). Output from fpc -Mtp, up to that error. }
type
   TBytes = array[0..255] of Byte;
   Fig = record
      X: Integer;
      case Kind: Integer of
         0: (R: Integer);
         1: (W, H: Integer);
   end;
var
   buf: array[1..10] of Byte;
   words: array[1..3] of Word;
   f: Fig;
   i: Integer;
   out: array[1..2] of Byte;
procedure Dump(var Data; Size: Word);
   var k: Integer;
   begin
      for k := 0 to Size - 1 do Write(TBytes(Data)[k], ' ');
      WriteLn;
   end;
begin
   for i := 1 to 10 do buf[i] := i;
   i := 3;
   Dump(buf[i], 2);
   words[1] := $0102; words[2] := $0304; words[3] := $0506;
   i := 2;
   Dump(words[i], 2);
   f.X := 1; f.Kind := 1; f.W := 7; f.H := 9;
   Dump(f.H, 2);
   Move(words[i], out, 2);
   WriteLn(out[1], ' ', out[2]);
   i := 0;
   Dump(buf[i], 1);
end.
