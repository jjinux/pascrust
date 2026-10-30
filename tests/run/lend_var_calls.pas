program LendVarCalls;
{ A routine that changes a Char array the statement uses as a PChar, as a
  method of its object or through a var argument, called before that
  use: it runs first, so the copy sees its change. Output from fpc -Mtp. }
{$X+}
uses Strings;
type
   Buf = array[0..31] of Char;
   TObj = object
      A: Buf;
      function Len: Integer;
   end;
   Rec = record
      A: Buf;
   end;
var
   o: TObj;
   r: Rec;
   k: Integer;
function TObj.Len: Integer;
   begin
      StrCat(A, 'x');
      Len := StrLen(A);
   end;
function Grow(var rr: Rec): Integer;
   begin
      StrCat(rr.A, 'y');
      Grow := StrLen(rr.A);
   end;
begin
   StrCopy(o.A, 'ab');
   k := o.Len + StrLen(o.A);
   WriteLn(k, ' ', o.A);
   StrCopy(r.A, 'ab');
   k := Grow(r) + StrLen(r.A);
   WriteLn(k, ' ', r.A);
end.
