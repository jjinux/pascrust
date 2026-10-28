program SystemVars;
{ The System unit's variables: RandSeed, ExitCode, FileMode, and Input
  and Output as files. Self-checking; ends with exit code 3 from ExitCode. }
var
  checks, failures: Integer;
  first, second: Integer;
  n: Integer;
  f: file of Integer;

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
  RandSeed := 1234;
  first := Random(1000);
  RandSeed := 1234;
  second := Random(1000);
  Check(first = second, 'RandSeed repeats a sequence');
  Check(RandSeed <> 1234, 'Random moves RandSeed on');
  Check(FileMode = 2, 'FileMode starts at 2');
  Assign(f, 'system_vars.tmp');
  Rewrite(f);
  n := 7;
  Write(f, n);
  Close(f);
  FileMode := 0;
  Reset(f);
  Read(f, n);
  Close(f);
  Erase(f);
  FileMode := 2;
  Check(n = 7, 'Reset with FileMode 0 reads');
  Write(Output, 'to ');
  WriteLn(Output, 'Output');
  ReadLn(Input, n);
  Check(n = 42, 'ReadLn(Input, ...)');
  Check(Eof(Input), 'Eof(Input)');
  Check(ExitCode = 0, 'ExitCode starts at 0');
  ExitCode := 3;
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
