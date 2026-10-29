program BlockFiles;
{ Untyped files: BlockWrite and BlockRead in records of 1 byte or of a
  record's size, a copy in chunks with a short last read, a string's
  characters (s[1]), Seek, FileSize, FilePos and Eof in records. Output
  from fpc -Mtp. }
type
  Rec = record
    name: string[7];
    n: Integer;
  end;
var
  f, g: file;
  buf: array[1..1000] of Byte;
  r, back: Rec;
  s: string;
  i, total: Integer;
  got, wrote: Word;
begin
  for i := 1 to 1000 do buf[i] := i mod 251;
  Assign(f, 'data.bin');
  Rewrite(f, 1);
  BlockWrite(f, buf, 1000, wrote);
  WriteLn('wrote ', wrote, ' size ', FileSize(f));
  Close(f);
  { copy in 256-byte chunks }
  Reset(f, 1);
  Assign(g, 'copy.bin');
  Rewrite(g, 1);
  total := 0;
  repeat
    BlockRead(f, buf, 256, got);
    BlockWrite(g, buf, got, wrote);
    total := total + got;
  until got = 0;
  WriteLn('copied ', total, ' size ', FileSize(g));
  Close(f);
  Close(g);
  { a record, with the default record size }
  r.name := 'pascal';
  r.n := 7;
  Rewrite(f, SizeOf(Rec));
  BlockWrite(f, r, 1);
  Close(f);
  Reset(f, SizeOf(Rec));
  BlockRead(f, back, 1);
  WriteLn(back.name, ' ', back.n, ' ', FileSize(f), ' ', FilePos(f), ' ', Eof(f));
  Close(f);
  { a string's characters }
  s := 'hello world';
  Rewrite(f, 1);
  BlockWrite(f, s[1], Length(s));
  Close(f);
  Reset(f, 1);
  WriteLn('text ', FileSize(f));
  Seek(f, 6);
  s := '';
  s[0] := Chr(5);
  BlockRead(f, s[1], 5);
  WriteLn(s);
  Close(f);
  Erase(f);
  Erase(g);
end.
