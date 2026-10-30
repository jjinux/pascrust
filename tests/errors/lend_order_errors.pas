program LendOrderErrors;
{ A routine that changes a Char array, called after the statement uses the
  array as a PChar: the write-back after the statement would undo it. }
{$X+}
uses Strings;
var
   Buf: array[0..15] of Char;

type
   Rec = record
      A: array[0..15] of Char;
   end;

var
   r: Rec;

function Grow(var rr: Rec): Integer;
   begin
      StrCat(rr.A, 'y');
      Grow := StrLen(rr.A);
   end;

function Fill: Integer;
   begin
      StrCopy(Buf, 'filled');
      Fill := 1;
   end;

begin
   WriteLn(StrLen(StrUpper(Buf)), ' ', Fill);
   WriteLn(StrLen(StrCat(r.A, '!')) + Grow(r));
end.
