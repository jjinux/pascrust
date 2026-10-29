program StdFilesVar;
{ Output, Input and the printer's Lst passed where a var Text parameter is
  expected: a common way to write a report once and send it anywhere.
  Self-checking. (Free Pascal's Lst is a real printer device, so this .out
  is built from the checks, by hand.) }
uses Printer;
var
  checks, failures: Integer;
  f: Text;
  line: string;
  n: Integer;

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

procedure Report(var dest: Text; total: Integer);
begin
  WriteLn(dest, 'Total: ', total);
end;

procedure ReadNumber(var src: Text; var value: Integer);
begin
  ReadLn(src, value);
end;

begin
  checks := 0;
  failures := 0;
  Report(Output, 1);
  Report(Lst, 2);
  Flush(Lst);
  Assign(f, 'printer.txt');
  Reset(f);
  ReadLn(f, line);
  Close(f);
  Check(line = 'Total: 2', 'Report(Lst) printed');
  ReadNumber(Input, n);
  Check(n = 42, 'ReadNumber(Input)');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
