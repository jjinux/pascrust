program StringOverlay;
{ A string[n] in a tagless variant part, as TP's memory: setting it
  changes only its length byte and characters, the bytes past its length
  stay, and its length byte can be set (s[0]) or overlaid with a Byte.
  Output from fpc -Mtp. }
type
   Pun = record
      case Integer of
         0: (L: LongInt);
         1: (S: string[3]);
         2: (Len: Byte; C: array[1..3] of Char);
   end;
var
   p: Pun;
begin
   p.L := $44434241;
   p.S := '';
   WriteLn(p.L, ' ', p.C[1], p.C[2], p.C[3]);
   p.L := $44434241;
   p.S[0] := #0;
   p.S[0] := #3;
   WriteLn(p.S);
   p.L := $44434241;
   Inc(p.S[0]);
   WriteLn(p.L);
   p.Len := 2;
   WriteLn(p.S, ' ', Length(p.S));
end.
