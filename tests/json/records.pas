{ Writes items.dat and raws.dat, the fixtures of crates/ruscal/tests/
  json_tests.rs: a record with every kind of field to-json reads, a tagged
  variant part, and a tagless one. An Item is 45 bytes, as in TP 7. (Free
  Pascal can't check this: on a 64-bit machine its pointers take 8 bytes,
  and its sets and records are laid out differently.) }
program Records;
type
   Color = (Red, Green, Blue);
   Shape = (Circle, Square);
   Point = record
      X, Y: Integer;
   end;
   ItemPtr = ^Item;
   Item = record
      Name: string[10];
      Count: Byte;
      Delta: ShortInt;
      Big: LongInt;
      Code: Word;
      Ok: Boolean;
      Letter: Char;
      Hue: Color;
      Hues: set of Color;
      Letters: set of 'a'..'z';
      Grid: array[1..2, 1..3] of Byte;
      Where: Point;
      Next: ItemPtr;
      case Kind: Shape of
         Circle: (Radius: Integer);
         Square: (Side: Word; Filled: Boolean);
   end;
   Raw = record
      case Integer of
         0: (W: Word);
         1: (Lo, Hi: Byte);
   end;
   ItemFile = file of Item;
   RawFile = file of Raw;
var
   Items: ItemFile;
   Raws: RawFile;
   It: Item;
   R: Raw;
   I, J: Integer;
begin
   Assign(Items, 'items.dat');
   Rewrite(Items);
   FillChar(It, SizeOf(It), 0);
   It.Name := 'Caf'#130;
   It.Count := 200;
   It.Delta := -5;
   It.Big := -100000;
   It.Code := 65535;
   It.Ok := True;
   It.Letter := 'Z';
   It.Hue := Blue;
   It.Hues := [Red, Blue];
   It.Letters := ['a', 'q', 'z'];
   for I := 1 to 2 do
      for J := 1 to 3 do
         It.Grid[I, J] := I * 10 + J;
   It.Where.X := -1;
   It.Where.Y := 300;
   It.Next := nil;
   It.Kind := Circle;
   It.Radius := 7;
   Write(Items, It);
   FillChar(It, SizeOf(It), 0);
   It.Name := 'Box';
   It.Kind := Square;
   It.Side := 12;
   It.Filled := True;
   Write(Items, It);
   Close(Items);
   Assign(Raws, 'raws.dat');
   Rewrite(Raws);
   R.W := 258;
   Write(Raws, R);
   Close(Raws);
end.
