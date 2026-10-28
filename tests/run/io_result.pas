program IOCheck;
{ With $I-, an I/O error sets IOResult instead of stopping the program. }
var
  f: Text;
  n: Integer;
  code: Integer;

function Exists(name: string): Boolean;
begin
  Assign(f, name);
  {$I-}
  Reset(f);
  {$I+}
  Exists := IOResult = 0;
end;

begin
  WriteLn('missing: ', Exists('no_such_file.txt'));
  code := IOResult;
  WriteLn('after reading it: ', code);
  { While an error waits to be read, more I/O does nothing. }
  {$I-}
  Reset(f);
  Close(f);
  {$I+}
  { Read IOResult before the next checked I/O: TP would stop on the error
    that's still waiting. }
  code := IOResult;
  WriteLn('close was skipped: ', code);
  {$I-}
  ReadLn(n);
  {$I+}
  code := IOResult;
  WriteLn('bad number: ', code);
  ReadLn(n);
  code := IOResult;
  WriteLn('good number: ', n, ' ', code);
end.
