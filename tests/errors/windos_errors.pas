program WinDosErrors;
{ WinDos: a PChar from FileExpand points into its first argument, so it
  can't be kept when that's a Char array (GetEnvVar's can); FindFirst takes
  a TSearchRec; Intr needs real DOS. }
uses WinDos;
var
  buf: array[0..79] of Char;
  p: PChar;
  n: Integer;
  r: TRegisters;
begin
  p := FileExpand(buf, 'x');
  p := GetEnvVar('PATH');
  FindFirst('*.*', faAnyFile, n);
  Intr($21, r);
end.
