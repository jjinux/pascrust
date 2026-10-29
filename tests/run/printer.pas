program PrinterTest;
{ The Printer unit's Lst. pascrust prints to printer.txt (or wherever
  PASCRUST_PRINTER says), so this reads the file back to check what was
  printed. Self-checking. (Free Pascal's Lst is a real printer device, so
  this .out is built from the checks, by hand.) }
uses Printer;
var
  checks, failures: Integer;
  f: Text;
  line: string;
  total: Real;

procedure Check(ok: Boolean; what: string);
begin
  checks := checks + 1;
  if ok then
    WriteLn('ok: ', what)
  else
  begin
    failures := failures + 1;
    WriteLn('FAILED: ', what);
  end;
end;

begin
  checks := 0;
  failures := 0;
  total := 12.5;
  WriteLn(Lst, 'Report for March');
  Write(Lst, 'Total: ');
  WriteLn(Lst, total:6:2);
  Flush(Lst);
  Assign(f, 'printer.txt');
  Reset(f);
  ReadLn(f, line);
  Check(line = 'Report for March', 'first printed line');
  ReadLn(f, line);
  Check(line = 'Total:  12.50', 'second printed line, with a format');
  Check(Eof(f), 'nothing else printed');
  Close(f);
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
