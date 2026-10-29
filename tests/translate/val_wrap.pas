program ValWrap;
{ Val reads a number as a 64-bit integer and stores it in the variable's
  type, wrapping if it doesn't fit; a minus sign is an error for an
  unsigned type, and after an error the variable is 0. TP left these
  cases undefined; this follows Free Pascal, whose output this is. }
var
  i, code: Integer;
  b: Byte;
  w: Word;
  l: LongInt;
begin
  Val('40000', i, code); WriteLn(i, ' ', code);
  Val('300', b, code); WriteLn(b, ' ', code);
  Val('70000', i, code); WriteLn(i, ' ', code);
  Val('$FFFF', i, code); WriteLn(i, ' ', code);
  Val('-1', w, code); WriteLn(w, ' ', code);
  Val('3000000000', l, code); WriteLn(l, ' ', code);
  Val('12x', i, code); WriteLn(i, ' ', code);
end.
