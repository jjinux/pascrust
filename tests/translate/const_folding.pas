program ConstFolding;
{ TP 7's constant functions, worked out when the program is translated,
  agree with the same calls at run time; Swap works in the type Free
  Pascal chooses; and enum ranges in set tests.
  Self-checking. (Free Pascal can't evaluate some of these constants, so
  this .out is built from the checks, by hand.) }
type
  Day = (Mon, Tue, Wed, Thu, Fri, Sat, Sun);
const
  Last = Sun;
  BeforeLast = Pred(Last);
  SwapNeg = Swap(-1);
  Swap255 = Swap(255);
  SwapWord = Swap($FF00);
  HiNeg = Hi(-256);
  LoNeg = Lo(-1);
  Len = Length('abcd');
  OddNeg = Odd(-3);
  SwapTwice = Swap(Swap($FF00));
  HighWord = $FF00;
  SwapNamed = Swap(HighWord);
var
  checks, failures: Integer;
  i: Integer;
  w: Word;
  b: Byte;
  sh: ShortInt;
  l: LongInt;
  d: Day;

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
  i := -1;
  Check(SwapNeg = Swap(i), 'Swap(-1) folded as at run time');
  w := 255;
  Check(Swap255 = Swap(w), 'Swap(255) works on a Word, as in Free Pascal');
  i := 255;
  Check(Swap(i) = -256, 'Swap of an Integer variable stays an Integer');
  b := 1;
  sh := -1;
  Check((Swap(b) = 256) and (Swap(sh) = -1), 'a Byte swaps as a Word, a ShortInt as an Integer');
  l := $12345678;
  Check(Swap(l) = $56781234, 'Swap of a LongInt trades its halves');
  Check(SwapWord = 255, 'Swap($FF00) is a Word');
  Check((SwapTwice = 65280) and (Swap(Swap($FF00)) = 65280), 'Swap of a Word stays a Word');
  Check((SwapNamed = 255) and (Swap(HighWord) = 255), 'Swap of a named Word constant');
  i := -5;
  Check(-(-i) = -5, 'a negated negation');
  i := -256;
  Check(HiNeg = Hi(i), 'Hi(-256)');
  i := -1;
  Check(LoNeg = Lo(i), 'Lo(-1)');
  Check((Hi(-1) = 255) and (Lo(-256) = 0), 'Hi and Lo of negative literals');
  Check(Len = 4, 'Length of a string constant');
  Check(OddNeg, 'Odd(-3)');
  Check(BeforeLast = Sat, 'Pred of an enum constant');
  d := Sun;
  Check(d in [Sat..Sun], 'an enum range in a set test');
  Check(not (Wed in [Sat..Last]), 'and one ending in a constant');
  Check(True in [False..True], 'a Boolean range');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
