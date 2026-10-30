program BlockFileIO;
{ Untyped files with I/O checks off: a short BlockRead without a Result count is
  error 100 in IOResult, not a stop; and Eof counts in records, so a
  part of a record at the end is at the end. Output from fpc -Mtp. }
var
   f: file;
   buf: array[1..400] of Byte;
   got: Word;
   n: Integer;
begin
   Assign(f, 'block_file_io.bin');
   Rewrite(f, 1);
   BlockWrite(f, buf, 300);
   Close(f);
   Reset(f, 1);
   {$I-}
   BlockRead(f, buf, 350);
   {$I+}
   n := IOResult;
   WriteLn('short read: ', n);
   Close(f);
   Reset(f);
   BlockRead(f, buf, 2, got);
   WriteLn('got ', got, ' pos ', FilePos(f), ' size ', FileSize(f), ' eof ', Eof(f));
   Close(f);
   Erase(f);
end.
