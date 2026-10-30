program StringPlaces;
{ Insert, Delete and FillChar on a string[n] field of a tagless variant
  part and on a string through a pointer, and a string[4] typecast of an
  untyped parameter set from a concatenation. Output from fpc -Mtp. }
type
   Pun = record
      case Integer of
         0: (L: LongInt);
         1: (S: string[3]);
   end;
   PStr = ^Str20;
   Str20 = string[20];
   TBytes = array[0..255] of Byte;
   S4 = string[4];
   Named = record
      Name: string[4];
      N: Integer;
   end;
var
   p: Pun;
   ps: PStr;
   t: string;
   nm: Named;
procedure SetName(var Data; const S: string);
   begin
      S4(Data) := S + 'z';
   end;
begin
   p.S := 'abc';
   Delete(p.S, 1, 1);
   Insert('q', p.S, 1);
   WriteLn(p.S);
   FillChar(p.S, 4, 0);
   WriteLn(Length(p.S));
   New(ps);
   ps^ := 'hello';
   Delete(ps^, 1, 1);
   Insert('j', ps^, 1);
   WriteLn(ps^);
   t := 'Hel';
   nm.N := 258;
   SetName(nm, t);
   WriteLn(nm.Name, ' ', nm.N);
end.
