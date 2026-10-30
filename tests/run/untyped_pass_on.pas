program UntypedPassOn;
{ An untyped parameter passed on from an offset (`TBytes(Data)[2]`) and
  through Move, and used by a nested procedure (which gets the outer
  routine's `Bytes` guard), changing it and passing it on. Output from
  fpc -Mtp. }
type
   TBytes = array[0..255] of Byte;
   Rec = record
      a, b: Word;
   end;
var
   r: Rec;
   w: Word;
   buf: array[1..4] of Byte;

procedure Dump(var Data; Size: Word);
   var
      i: Integer;
   begin
      for i := 0 to Size - 1 do
         Write(TBytes(Data)[i], ' ');
      WriteLn;
   end;

procedure Tail(var Data; Size: Word);
   begin
      Dump(TBytes(Data)[2], Size - 2);
      Move(TBytes(Data)[1], buf, 2);
      WriteLn(buf[1], ' ', buf[2]);
   end;

procedure Outer(var Data; Size: Word);

   procedure Inner;
      begin
         TBytes(Data)[0] := 42;
         Dump(Data, Size);
      end;

   begin
      Inner;
      Dump(Data, Size);
   end;

begin
   r.a := $0102;
   r.b := $0304;
   Tail(r, 4);
   Outer(r, 4);
   w := $AABB;
   Outer(w, 2);
   WriteLn(r.a, ' ', w);
end.
