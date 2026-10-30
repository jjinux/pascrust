program SizedStorage;
{ Strings and small sets keep their TP sizes outside a record's direct
  fields too: a file of string[10] (11 bytes a record), a file of an
  array of string[5], an array of string[4] and an array of small sets as
  untyped bytes and through Move, and a record field that's an array of
  small sets in a file. Output from fpc -Mtp. }
type
   Arr = array[1..2] of string[5];
   Color = (Red, Green, Blue);
   TBytes = array[0..255] of Byte;
   NameArr = array[1..3] of string[4];
   SetArr = array[1..2] of set of Color;
   Flags = record
      Marks: SetArr;
   end;
var
   f: file of string[10];
   g: file of Arr;
   a: Arr;
   s: string[10];
   b: file of Byte;
   names: NameArr;
   sets: SetArr;
   buf: array[1..15] of Byte;
   i: Integer;
   fl: file of Flags;
   fr: Flags;
procedure Dump(var Data; Size: Word);
   var i: Integer;
   begin
      for i := 0 to Size - 1 do Write(TBytes(Data)[i], ' ');
      WriteLn;
   end;
begin
   Assign(f, 'fs.dat'); Rewrite(f); s := 'hello'; Write(f, s); s := 'world'; Write(f, s); Close(f);
   Assign(b, 'fs.dat'); Reset(b); WriteLn('bytes ', FileSize(b)); Close(b);
   Reset(f); Seek(f, 1); Read(f, s); Close(f); WriteLn(s);
   a[1] := 'ab'; a[2] := 'cde'; Assign(g, 'fa.dat'); Rewrite(g); Write(g, a); Close(g);
   Assign(b, 'fa.dat'); Reset(b); WriteLn('arr bytes ', FileSize(b), ' sizeof ', SizeOf(Arr)); Close(b);
   names[1] := 'ab'; names[2] := 'c'; names[3] := 'defg';
   Dump(names, SizeOf(names));
   Dump(names[2], 10);
   Move(names, buf, 15);
   for i := 1 to 15 do Write(buf[i], ' ');
   WriteLn;
   sets[1] := [Red]; sets[2] := [Blue];
   Dump(sets, SizeOf(sets));
   fr.Marks := sets;
   Assign(fl, 'fl.dat'); Rewrite(fl); Write(fl, fr); Close(fl);
   Assign(b, 'fl.dat'); Reset(b); WriteLn('flags bytes ', FileSize(b)); Close(b);
   Reset(fl); Read(fl, fr); Close(fl); WriteLn(Blue in fr.Marks[2], ' ', Red in fr.Marks[2]);
   Erase(f); Erase(g); Erase(fl);
end.
