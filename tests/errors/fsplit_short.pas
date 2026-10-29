program FSplitShort;
{ FSplit's results need room: TP wants DirStr, NameStr and ExtStr. }
uses Dos;
var
  d: DirStr;
  n: string[3];
  e: ExtStr;
begin
  FSplit('C:\X\ABCDEFGH.TXT', d, n, e);
end.
