program CaseLabels;
{ Case labels: wide ranges, ranges that cover a whole integer type, enum
  ranges, and labels an earlier arm already covers. Self-checking. (Free
  Pascal rejects a label that appears twice, which TP allows, so this .out
  is built from the checks, by hand.) }
type
  Day = (Mon, Tue, Wed, Thu, Fri, Sat, Sun);
var
  checks, failures: Integer;
  l: LongInt;
  i: Integer;
  w: Word;
  sh: ShortInt;
  b: Byte;
  d: Day;
  what: string;

procedure Check(ok: Boolean; what: string);
begin
  checks := checks + 1;
  if ok then
    WriteLn('ok: ', what)
  else
  begin
    failures := failures + 1;
    WriteLn('FAILED: ', what);
  end;
end;

begin
  checks := 0;
  failures := 0;
  l := 5;
  case l of
    1..1000000000: what := 'big';
  end;
  Check(what = 'big', 'a range of a billion values');
  i := -7;
  case i of
    -32768..-1: what := 'negative';
    0..32767: what := 'not negative';
  end;
  Check(what = 'negative', 'two ranges cover every Integer');
  w := 65535;
  case w of
    0..65535: what := 'word';
  end;
  Check(what = 'word', 'one range covers every Word');
  sh := -128;
  case sh of
    -128..127: what := 'shortint';
  else
    what := 'never';
  end;
  Check(what = 'shortint', 'a ShortInt case with an else that can''t run');
  d := Wed;
  case d of
    Mon..Fri: what := 'weekday';
    Sat, Sun: what := 'weekend';
  end;
  Check(what = 'weekday', 'an enum range: Mon..Fri');
  d := Sun;
  case d of
    Mon..Fri: what := 'weekday';
  else
    what := 'weekend';
  end;
  Check(what = 'weekend', 'an enum range with an else');
  b := 2;
  case b of
    1, 2: what := 'one or two';
    1..2: what := 'never';
    0: what := 'zero';
  else
    what := 'more';
  end;
  Check(what = 'one or two', 'a range an earlier arm covers: the first arm wins');
  b := 9;
  case b of
    3..7: what := 'middle';
    1..5, 6..10: what := 'low';
  end;
  Check(what = 'low', 'ranges that overlap an earlier one');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
