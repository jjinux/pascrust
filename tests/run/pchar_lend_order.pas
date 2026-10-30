program PCharLendOrder;
{ A statement that reads a Char array through its PChar before it writes
  through it: the write still reaches the array, in an assignment and in
  `and`/`or` conditions. Output from fpc -Mtp. }
{$X+}
uses Strings;
var
   buf: array[0..15] of Char;
   i: Integer;
begin
   StrCopy(buf, 'ab');
   i := StrLen(buf) + StrLen(StrCat(buf, 'q'));
   WriteLn(i, ' ', StrPas(buf));
   StrCopy(buf, 'xxx');
   if (StrLen(buf) = 3) and (StrComp(StrUpper(buf), 'XXX') = 0) then
      WriteLn('and ', StrPas(buf));
   if (StrLen(buf) = 0) or (StrLen(StrCat(buf, 'y')) = 4) then
      WriteLn('or ', StrPas(buf));
   while (StrLen(buf) < 6) and (StrLen(StrCat(buf, 'z')) > 0) do
      Write('.');
   WriteLn(' ', StrPas(buf));
end.
