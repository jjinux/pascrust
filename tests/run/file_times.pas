program FileTimes;
{ File times and attributes: PackTime/UnpackTime, GetFTime/SetFTime and
  GetFAttr/SetFAttr. Self-checking. (Free Pascal on Unix can't remember
  Hidden or Archive, which pascrust does while the program runs, so this
  .out is built from the checks, by hand.) }
uses Dos;
var
  checks, failures: Integer;
  dt, back: DateTime;
  whenPacked, stamp: LongInt;
  f: Text;
  g: file of Integer;
  attr: Word;
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

begin
  checks := 0;
  failures := 0;
  dt.Year := 1997;
  dt.Month := 3;
  dt.Day := 11;
  dt.Hour := 7;
  dt.Min := 30;
  dt.Sec := 41;
  PackTime(dt, whenPacked);
  UnpackTime(whenPacked, back);
  Check((back.Year = 1997) and (back.Month = 3) and (back.Day = 11), 'PackTime and UnpackTime keep the date');
  Check((back.Hour = 7) and (back.Min = 30) and (back.Sec = 40), 'and the time, to 2 seconds');
  Assign(f, 'stamp.txt');
  Rewrite(f);
  WriteLn(f, 'hello');
  Close(f);
  SetFTime(f, whenPacked);
  Check(DosError = 0, 'SetFTime');
  GetFTime(f, stamp);
  Check(stamp = whenPacked, 'GetFTime reads back what SetFTime set');
  GetFAttr(f, attr);
  Check((attr and Archive <> 0) and (attr and Directory = 0), 'a new file is Archive, not a Directory');
  SetFAttr(f, ReadOnly);
  GetFAttr(f, attr);
  Check(attr and ReadOnly <> 0, 'SetFAttr ReadOnly');
  SetFAttr(f, Hidden);
  GetFAttr(f, attr);
  Check((attr and Hidden <> 0) and (attr and ReadOnly = 0), 'SetFAttr Hidden, and not ReadOnly any more');
  Erase(f);
  Assign(g, 'numbers.dat');
  Rewrite(g);
  n := 5;
  Write(g, n);
  Close(g);
  GetFTime(g, stamp);
  Check((DosError = 0) and (stamp <> 0), 'GetFTime of a typed file');
  Erase(g);
  MkDir('sub');
  Assign(f, 'sub');
  GetFAttr(f, attr);
  Check(attr and Directory <> 0, 'GetFAttr of a directory');
  RmDir('sub');
  Assign(f, 'missing.txt');
  GetFAttr(f, attr);
  Check(DosError = 2, 'GetFAttr of a missing file: DosError 2');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
