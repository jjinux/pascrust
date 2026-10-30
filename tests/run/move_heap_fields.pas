{ Move between two fields of one heap record, of different types: the
  source's bytes are read before the record is changed. }
program MoveHeapFields;
type
  PRec = ^TRec;
  TRec = record
    w: Word;
    b: array[0..1] of Byte;
    l: LongInt;
  end;
var
  p: PRec;
begin
  New(p);
  p^.w := $0102;
  if p^.w > 0 then
    Move(p^.w, p^.b, 1);
  p^.l := 0;
  Move(p^.w, p^.b, 2);
  WriteLn(p^.b[0], ' ', p^.b[1]);
  Move(p^.w, p^.l, 2);
  WriteLn(p^.l);
  FillChar(p^.b, 2, 7);
  Move(p^.b, p^.w, 2);
  WriteLn(p^.w);
end.
