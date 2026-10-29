program SetFile;
{ Sets as fields of records in a typed file: written and read back. }
type
  Rec = record
    n: Integer;
    flags: set of 0..7;
    letters: set of Char;
  end;
var
  f: file of Rec;
  r: Rec;
begin
  r.n := 5;
  r.flags := [1, 3];
  r.letters := ['a'..'c'];
  Assign(f, 'setfile.dat');
  Rewrite(f);
  Write(f, r);
  Close(f);
  r.flags := [];
  r.letters := [];
  Reset(f);
  Read(f, r);
  Close(f);
  Erase(f);
  WriteLn(r.n, ' ', 3 in r.flags, ' ', 2 in r.flags, ' ', 'b' in r.letters, ' ', 'z' in r.letters);
end.
