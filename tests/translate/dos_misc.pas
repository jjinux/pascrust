program DosMisc;
{ The smaller Dos routines: FSplit, EnvCount/EnvStr, DosError as a
  variable, and the ones PascRust fakes (DosVersion, DiskFree/DiskSize,
  SetDate/SetTime, the Ctrl-Break and verify switches, SwapVectors).
  Self-checking. (Free Pascal reports the real system's version and disk
  space, so this .out is built from the checks, by hand.) }
uses Dos;
var
  checks, failures: Integer;
  d: DirStr;
  n: NameStr;
  e: ExtStr;
  b: Boolean;
  v: Word;
  r: Registers;

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
  FSplit('C:\GAMES\SCORES.DAT', d, n, e);
  Check((d = 'C:\GAMES\') and (n = 'SCORES') and (e = '.DAT'), 'FSplit of a full path');
  FSplit('README', d, n, e);
  Check((d = '') and (n = 'README') and (e = ''), 'FSplit of a bare name');
  FSplit('A:LONGFILENAME.TEXT', d, n, e);
  Check((d = 'A:') and (n = 'LONGFILE') and (e = '.TEX'), 'FSplit cuts to NameStr and ExtStr');
  Check(EnvCount > 0, 'EnvCount');
  Check(Pos('=', EnvStr(1)) > 1, 'EnvStr is NAME=value');
  Check(EnvStr(0) = '', 'EnvStr(0) is empty');
  v := DosVersion;
  Check((Lo(v) = 6) and (Hi(v) = 22), 'DosVersion is 6.22');
  Check((DiskSize(0) > 0) and (DiskFree(0) > 0) and (DiskFree(0) <= DiskSize(0)), 'DiskFree and DiskSize');
  Check(DiskFree(0) div (DiskSize(0) div 100) = 50, 'DiskFree is half of DiskSize');
  Check(DiskSize(30) = -1, 'DiskSize of an invalid drive');
  SetCBreak(True);
  GetCBreak(b);
  Check(b, 'SetCBreak and GetCBreak');
  SetVerify(True);
  GetVerify(b);
  Check(b, 'SetVerify and GetVerify');
  SetDate(1997, 3, 11);
  SetTime(7, 0, 0, 0);
  SwapVectors;
  r.AX := $1234;
  Check(r.AX = $1234, 'a Registers record can be declared and used');
  DosError := 5;
  Check(DosError = 5, 'DosError is a variable');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
