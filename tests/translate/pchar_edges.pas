program PCharEdges;
{ Edge cases of the Strings unit: StrPos of an empty string is the end,
  StrLen(nil) is 0, and the StrL... routines look at no more than MaxLen
  characters (a string without a #0 within them is fine). Output from
  fpc -Mtp. }
{$X+}
uses Strings;
var s, p, q: PChar; buf: array[0..9] of Char;
begin
   s := StrNew('abc');
   p := StrPos(s, '');
   WriteLn(p = nil, ' ', p - s);
   q := nil;
   WriteLn(StrLen(q));
   GetMem(p, 4); p[0] := 'x'; p[1] := 'y'; p[2] := 'z'; p[3] := 'w';
   StrLCopy(buf, p, 2);
   WriteLn('[', buf, '] ', StrLComp(p, 'xyq', 2), ' ', StrLIComp(p, 'XYZW', 4));
   StrCopy(buf, 'ab');
   StrLCat(buf, p, 4);
   WriteLn('[', buf, ']');
end.
