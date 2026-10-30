program LendCalls;
{ A statement that calls a routine changing a Char array, before it uses
  the array as a PChar: the routine runs first, so its change is seen and
  kept. And the statement's other reads of the array (Write, an element)
  see what it wrote through the PChar. Output from fpc -Mtp. }
{$X+}
uses Strings;
var
   buf: array[0..15] of Char;
   n: Integer;
function Fill: Integer;
   begin
      StrCopy(buf, 'filled');
      Fill := 1;
   end;
function Direct: Integer;
   begin
      buf[0] := 'D';
      Direct := 2;
   end;
begin
   StrCopy(buf, 'start');
   WriteLn(Fill, ' ', StrPas(buf));
   StrCopy(buf, 'start');
   WriteLn(Fill, ' ', StrLen(StrUpper(buf)));
   WriteLn(buf);
   StrCopy(buf, 'start');
   WriteLn(Direct, ' ', StrLen(StrUpper(buf)));
   WriteLn(buf);
   StrCopy(buf, 'xyz');
   WriteLn('22 ', StrLCopy(buf, 'abcdef', 2) = buf, ' [', buf, '] ', buf[1]);
   StrCopy(buf, 'hi');
   WriteLn('[', StrCat(buf, '!'), '] [', buf:5, ']');
   WriteLn(buf);
end.
