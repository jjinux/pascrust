{ Str into a string[n] cuts the text to fit, as an assignment does.
  Output from fpc -Mtp except lines 1 and 3: Free Pascal cuts an integer
  before padding it ('  1'), and keeps all of a Real's '  3.25' even in a
  string[4]; PascRust formats the whole field, then keeps its start. }
program StrShort;
var
  s: string[3];
  r: string[4];
begin
  Str(1234:5, s);
  WriteLn('[', s, '] ', Length(s));
  Str(7:2, s);
  WriteLn('[', s, '] ', Length(s));
  Str(3.25:6:2, r);
  WriteLn('[', r, '] ', Length(r));
  Str(-98765, r);
  WriteLn('[', r, ']');
end.
