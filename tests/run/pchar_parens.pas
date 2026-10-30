program PCharParens;
{ `(P + 1)^` and `(P + 1)[i]`, read and assigned: a selector after a
  parenthesized PChar expression (the TP 7 Language Guide's `P[I]` is
  `(P + I)^`). Output from fpc -Mtp. }
{$X+}
uses Strings;
var p: PChar; c: Char;
begin
   p := StrNew('abc');
   c := (p + 1)^;
   WriteLn(c, (p + 1)[1]);
   (p + 1)^ := 'X';
   (p + 1)[1] := 'Y';
   WriteLn(p);
end.
