program FillMoveParts;
{ Move between two fields of one heap record (the source is read into a
  `let` first, so the record isn't borrowed twice), and FillChar on an
  element of an array of strings. Output from fpc -Mtp. }
type
   PRec = ^Rec;
   Rec = record
      a, b: array[1..4] of Byte;
   end;
var
   p: PRec;
   i: Integer;
   names: array[1..3] of string[4];
begin
   New(p);
   for i := 1 to 4 do
   begin
      p^.a[i] := i;
      p^.b[i] := 0;
   end;
   Move(p^.a, p^.b, 4);
   for i := 1 to 4 do
      Write(p^.b[i], ' ');
   WriteLn;
   for i := 1 to 3 do
      names[i] := 'abcd';
   FillChar(names[2], 5, 0);
   WriteLn(Length(names[1]), ' ', Length(names[2]), ' [', names[2], '] ', names[3]);
end.
