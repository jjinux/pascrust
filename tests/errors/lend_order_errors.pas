program LendOrderErrors;
{ A routine that changes a Char array, called after the statement uses the
  array as a PChar: the write-back after the statement would undo it. }
{$X+}
uses Strings;
var
  buf: array[0..15] of Char;

function Fill: Integer;
begin
  StrCopy(buf, 'filled');
  Fill := 1;
end;

begin
  WriteLn(StrLen(StrUpper(buf)), ' ', Fill);
end.
