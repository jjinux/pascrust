program FindFiles;
{ FindFirst and FindNext with DOS wildcards and attributes. Self-checking.
  (Free Pascal returns matches in directory order, so this .out is built
  from the checks, by hand: PascRust sorts them by name.) }
uses Dos;
var
  checks, failures: Integer;
  s, t: SearchRec;
  f: Text;
  found: string;
  count, i, existing, walkedFiles, walkedDirs: Integer;
  number, dir: string;

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

function Exists(name: string): Boolean;
var
  t: SearchRec;
begin
  FindFirst(name, AnyFile, t);
  Exists := DosError = 0;
end;

{ A recursive directory walk: each level's search waits while the levels
  below are searched. }
procedure Walk(path: string);
var
  t: SearchRec;
begin
  FindFirst(path + '\*.*', AnyFile, t);
  while DosError = 0 do
  begin
    if t.Attr and Directory = 0 then
      walkedFiles := walkedFiles + 1
    else if (t.Name <> '.') and (t.Name <> '..') then
    begin
      walkedDirs := walkedDirs + 1;
      Walk(path + '\' + t.Name);
    end;
    FindNext(t);
  end;
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
  Check(count = 3, 'with Directory, *.* finds the subdirectory, . and ..');
  FindFirst('sub\*.*', Directory, s);
  found := s.Name;
  FindNext(s);
  found := found + ' ' + s.Name;
  FindNext(s);
  Check((found = '. ..') and (DosError = 18), 'an empty subdirectory has . and ..');
  FindFirst('*.TXT', Directory, s);
  Check(s.Name = 'a.txt', 'a pattern . doesn''t match leaves it out');
  FindFirst('*.*', Archive, s);
  count := 0;
  while DosError = 0 do
  begin
    if s.Attr and Directory <> 0 then
      count := count + 1;
    FindNext(s);
  end;
  Check(count = 0, 'without Directory, no directories');
  FindFirst('*.TXT', AnyFile, s);
  t := s;
  FindNext(t);
  FindNext(s);
  Check((t.Name = 'b.txt') and (s.Name = 'b.txt'), 'a copy of a SearchRec goes on by itself');
  FindFirst('*.TXT', AnyFile, s);
  t := s;
  FindFirst('b.txt', AnyFile, t);
  FindNext(s);
  Check((DosError = 0) and (s.Name = 'b.txt'), 'FindFirst on a copy leaves the original''s search alone');
  FindFirst('A:*.*', Directory, s);
  Check(s.Name <> '.', 'no . in the root of a drive');
  FindFirst('A:\sub\..\*.*', Directory, s);
  Check(s.Name <> '.', 'nor when the root is reached through ..');
  FindFirst('A:\..\*.*', Directory, s);
  Check(s.Name <> '.', '.. stops at the root of a drive');
  MkDir('tree');
  Make('tree\a.txt', 'a');
  MkDir('tree\sub');
  for i := 1 to 150 do
  begin
    Str(i, number);
    MkDir('tree\sub\d' + number);
  end;
  Make('tree\z.txt', 'z');
  walkedFiles := 0;
  walkedDirs := 0;
  Walk('tree');
  Check((walkedFiles = 2) and (walkedDirs = 151), 'a walk through 151 directories sees both files');
  for i := 1 to 150 do
  begin
    Str(i, number);
    RmDir('tree\sub\d' + number);
  end;
  RmDir('tree\sub');
  Assign(f, 'tree\a.txt');
  Erase(f);
  Assign(f, 'tree\z.txt');
  Erase(f);
  RmDir('tree');
  MkDir('many');
  for i := 1 to 120 do
  begin
    Str(i, number);
    Make('many\f' + number + '.txt', 'x');
  end;
  count := 0;
  existing := 0;
  FindFirst('many\*.txt', AnyFile, s);
  while DosError = 0 do
  begin
    count := count + 1;
    if Exists('many\' + s.Name) then
      existing := existing + 1;
    FindNext(s);
  end;
  Check((count = 120) and (existing = 120), 'a search goes on while 120 others start and stop');
  for i := 1 to 120 do
  begin
    Str(i, number);
    Assign(f, 'many\f' + number + '.txt');
    Erase(f);
  end;
  RmDir('many');
  FindFirst('B?.TXT', AnyFile, s);
  Check((DosError = 0) and (s.Name = 'b.txt'), '? matches nothing at the end of a name');
  FindFirst('????????.???', Archive, s);
  Check((DosError = 0) and (s.Name = 'a.txt'), '????????.??? matches every name');
  FindFirst('*.*', VolumeID, s);
  Check(DosError = 18, 'VolumeID alone: no volume label');
  FindFirst('*.TXT', AnyFile, s);
  FindFirst('nowhere\*.*', AnyFile, s);
  FindNext(s);
  Check(DosError = 18, 'a failed FindFirst ends the search before it');
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
  { A:X is relative to drive A's current directory, as in DOS }
  Make('sub\inner.txt', 'in');
  ChDir('A:\sub');
  FindFirst('A:*.TXT', AnyFile, s);
  Check((DosError = 0) and (s.Name = 'inner.txt'), 'A:*.TXT after ChDir(''A:\sub'')');
  FindFirst('A:*.*', Directory, s);
  Check(s.Name = '.', 'and A: is no longer the root, so . is listed');
  GetDir(1, dir);
  Check(dir = 'A:\SUB', 'GetDir of drive A');
  Check(FExpand('A:x.dat') = 'A:\SUB\X.DAT', 'FExpand starts from the drive''s current directory');
  Check(FExpand('A:..\x.dat') = 'A:\X.DAT', 'and .. goes up from it');
  Assign(f, 'A:inner.txt');
  Erase(f);
  ChDir('A:\');
  RmDir('sub');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
