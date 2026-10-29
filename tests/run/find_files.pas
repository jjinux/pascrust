program FindFiles;
{ FindFirst and FindNext with DOS wildcards and attributes. Self-checking.
  (Free Pascal returns matches in directory order, so this .out is built
  from the checks, by hand: pascrust sorts them by name.) }
uses Dos;
var
  checks, failures: Integer;
  s: SearchRec;
  f: Text;
  found: string;
  count: Integer;

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

procedure Make(name: string; text: string);
begin
  Assign(f, name);
  Rewrite(f);
  Write(f, text);
  Close(f);
end;

begin
  checks := 0;
  failures := 0;
  Make('b.txt', 'bee');
  Make('a.txt', 'a');
  Make('notes.pas', 'program X; begin end.');
  MkDir('sub');
  found := '';
  count := 0;
  FindFirst('*.TXT', AnyFile, s);
  while DosError = 0 do
  begin
    found := found + s.Name + ' ';
    count := count + 1;
    FindNext(s);
  end;
  Check(found = 'a.txt b.txt ', 'FindFirst *.TXT finds both, in name order');
  Check(DosError = 18, 'DosError 18 after the last one');
  FindFirst('b.txt', AnyFile, s);
  Check((DosError = 0) and (s.Size = 3) and (s.Attr and Directory = 0), 'Size and Attr of a file');
  Check(s.Time <> 0, 'Time of a file');
  FindFirst('*.*', Directory, s);
  count := 0;
  while DosError = 0 do
  begin
    if s.Attr and Directory <> 0 then
      count := count + 1;
    FindNext(s);
  end;
  Check(count = 1, 'with Directory, *.* finds the subdirectory');
  FindFirst('*.*', Archive, s);
  count := 0;
  while DosError = 0 do
  begin
    if s.Attr and Directory <> 0 then
      count := count + 1;
    FindNext(s);
  end;
  Check(count = 0, 'without Directory, no directories');
  FindFirst('*.zzz', AnyFile, s);
  Check(DosError = 18, 'no match: DosError 18');
  FindFirst('nowhere\*.*', AnyFile, s);
  Check(DosError = 3, 'missing directory: DosError 3');
  Assign(f, 'b.txt');
  Erase(f);
  Assign(f, 'a.txt');
  Erase(f);
  Assign(f, 'notes.pas');
  Erase(f);
  RmDir('sub');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
