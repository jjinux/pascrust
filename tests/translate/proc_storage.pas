program ProcStorage;
{ A record with a procedural field in a typed file and as untyped bytes:
  the routine is 4 zero bytes, as TP's far address meant nothing to a
  later run of the program, and reads back as nil. Written by hand: Free
  Pascal stores the routine's real (8-byte) address. }
type
   Op = function(A, B: Integer): Integer;
   Entry = record
      Code: Integer;
      Apply: Op;
   end;
   TBytes = array[0..255] of Byte;
var
   e, back: Entry;
   f: file of Entry;

function Add(A, B: Integer): Integer;
   begin
      Add := A + B;
   end;

procedure Dump(var Data; Size: Word);
   var
      i: Integer;
   begin
      for i := 0 to Size - 1 do
         Write(TBytes(Data)[i], ' ');
      WriteLn;
   end;

begin
   e.Code := 258;
   e.Apply := Add;
   WriteLn(e.Apply(2, 3), ' ', SizeOf(Entry));
   Dump(e, SizeOf(e));
   Assign(f, 'proc_storage.dat');
   Rewrite(f);
   Write(f, e);
   Close(f);
   Reset(f);
   Read(f, back);
   Close(f);
   Erase(f);
   WriteLn(back.Code, ' ', Assigned(back.Apply), ' ', Assigned(e.Apply));
end.
