{ Str and Val between two fields of one heap record: the text is made
  (or read) first, so the record isn't read and written at once. }
program StrValHeap;
type
  PNode = ^TNode;
  TNode = record
    v: Integer;
    name: string[4];
  end;
var
  p: PNode;
  code: Integer;
begin
  New(p);
  p^.v := 7;
  Str(p^.v:2, p^.name);
  WriteLn('[', p^.name, ']');
  p^.name := '42';
  Val(p^.name, p^.v, code);
  WriteLn(p^.v, ' ', code);
  Dispose(p);
end.
