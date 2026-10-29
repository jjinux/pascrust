program SystemMisc;
{ GetDir, and the System routines pascrust fakes: MemAvail, MaxAvail,
  Mark, Release, SetTextBuf, Test8087. Self-checking. (GetDir gives
  pascrust's DOS paths, where drive C: is where the program started, so
  this .out is written by hand.) }
type
  PNode = ^Node;
  Node = record
    value: Integer;
  end;

var
  checks, failures: Integer;
  dir: string;
  short: string[4];
  heap, p: PNode;
  f: Text;
  buf: array[1..1024] of Char;

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
  GetDir(0, dir);
  Check(dir = 'C:\', 'GetDir at the start');
  MkDir('gd_tmp');
  ChDir('gd_tmp');
  GetDir(0, dir);
  Check(dir = 'C:\GD_TMP', 'GetDir after ChDir');
  GetDir(0, short);
  Check(short = 'C:\G', 'GetDir into a string[4]');
  ChDir('..');
  RmDir('gd_tmp');
  ChDir('..');
  GetDir(0, dir);
  Check(dir = 'C:\', 'GetDir above drive C: is C:\');
  ChDir('C:\');
  GetDir(1, dir);
  Check(dir = 'A:\', 'GetDir of drive A:');
  Check(MemAvail > 100000, 'MemAvail');
  Check((MaxAvail > 0) and (MaxAvail <= MemAvail), 'MaxAvail');
  Check(Test8087 = 3, 'Test8087');
  Mark(heap);
  New(p);
  p^.value := 1;
  Release(heap);
  Check(p^.value = 1, 'Release frees nothing');
  Assign(f, 'misc.tmp');
  SetTextBuf(f, buf);
  Rewrite(f);
  WriteLn(f, 'hi');
  Close(f);
  Erase(f);
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
