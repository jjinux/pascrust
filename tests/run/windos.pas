program WinDosTest;
{ The WinDos unit: the Dos routines with PChar strings. File names
  (FileSplit, FileExpand, FileSearch), directories (CreateDir, SetCurDir,
  GetCurDir, RemoveDir, with DosError), FindFirst/FindNext with
  TSearchRec, arguments and the environment, TDateTime, and the
  constants. Self-checking: Free Pascal on this machine has no WinDos, so
  this .out is built from the checks, by hand. }
uses WinDos, Strings;
var
   checks, failures: Integer;
   buf, dir, name, ext: array[0..79] of Char;
   s: string;
   flags: Word;
   sr: TSearchRec;
   dt, back: TDateTime;
   stamp: LongInt;

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
   flags := FileSplit('C:\DIR\NAME.EXT', dir, name, ext);
   Check(flags = fcDirectory or fcFileName or fcExtension, 'FileSplit flags');
   Check((StrComp(dir, 'C:\DIR\') = 0) and (StrComp(name, 'NAME') = 0) and (StrComp(ext, '.EXT') = 0), 'FileSplit parts');
   flags := FileSplit('*.PAS', nil, name, nil);
   Check((flags = fcFileName or fcExtension or fcWildcards) and (StrComp(name, '*') = 0), 'FileSplit wildcards, nil parts');
   FileExpand(buf, 'foo.txt');
   Check((StrLComp(buf, 'C:\', 3) = 0) and (StrComp(StrEnd(buf) - 8, '\FOO.TXT') = 0), 'FileExpand');
   Check(StrLen(FileSearch(buf, 'no_such_file.xyz', '')) = 0, 'FileSearch, not found');
   CreateDir('wd_test_dir');
   Check(DosError = 0, 'CreateDir');
   CreateDir('wd_test_dir');
   Check(DosError = 5, 'CreateDir of an existing directory: DosError 5');
   FindFirst('wd_test_dir', faDirectory, sr);
   Check((DosError = 0) and (StrComp(sr.name, 'wd_test_dir') = 0) and (sr.Attr and faDirectory <> 0), 'FindFirst with TSearchRec');
   CreateDir('wd_a_rather_long_directory_name');
   FindFirst('wd_a_rather_long_dir*', faDirectory, sr);
   Check(StrComp(sr.name, 'wd_a_rather_long_directory_name') = 0, 'TSearchRec keeps a long name whole');
   RemoveDir('wd_a_rather_long_directory_name');
   FindNext(sr);
   Check(DosError = 18, 'FindNext: no more files');
   FindFirst('*.no_such_ext', faAnyFile, sr);
   Check(DosError = 18, 'FindFirst: none');
   GetDir(0, s);
   Check(StrPas(GetCurDir(buf, 0)) = s, 'GetCurDir is GetDir');
   SetCurDir('wd_test_dir');
   Check((DosError = 0) and (StrComp(StrEnd(GetCurDir(buf, 0)) - 12, '\WD_TEST_DIR') = 0), 'SetCurDir (GetCurDir in capitals, as DOS)');
   SetCurDir('..');
   RemoveDir('wd_test_dir');
   Check(DosError = 0, 'RemoveDir');
   RemoveDir('wd_test_dir');
   Check(DosError = 3, 'RemoveDir of a missing directory: DosError 3');
   SetCurDir('no_such_dir');
   Check(DosError = 3, 'SetCurDir of a missing directory: DosError 3');
   Check(GetArgCount = ParamCount, 'GetArgCount');
   Check(StrLen(GetArgStr(buf, 0, 3)) = 3, 'GetArgStr with MaxLen');
   Check(GetEnvVar('PASCRUST_NO_SUCH_VARIABLE') = nil, 'GetEnvVar, not set');
   Check(StrComp(GetEnvVar('PASCRUST_DRIVE_C'), '.') = 0, 'GetEnvVar');
   dt.Year := 1994;
   dt.Month := 3;
   dt.Day := 17;
   dt.Hour := 13;
   dt.Min := 45;
   dt.Sec := 30;
   PackTime(dt, stamp);
   UnpackTime(stamp, back);
   Check((back.Year = 1994) and (back.Min = 45) and (back.Sec = 30), 'PackTime and UnpackTime with TDateTime');
   Check((fsPathName = 79) and (fmInOut = $D7B3) and (faAnyFile = $3F) and (fCarry = 1), 'constants');
   WriteLn(checks - failures, ' of ', checks, ' checks passed');
   if failures > 0 then
      Halt(1);
end.
