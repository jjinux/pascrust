program CaseLabelRanges;
{ Case labels the value can never match: TP rejects them. }
type
  Day = (Mon, Tue, Wed, Thu, Fri);
var
  b: Byte;
  i: Integer;
  d: Day;
begin
  case b of
    300: WriteLn('too big for a Byte');
    -1: WriteLn('a Byte is never negative');
  end;
  case i of
    5..1: WriteLn('backwards');
    40000: WriteLn('too big for an Integer');
  end;
  case d of
    Fri..Mon: WriteLn('backwards');
  end;
end.
