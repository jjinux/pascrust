program CrtVars;
{ Crt's variables: TextAttr, LastMode, CheckBreak and the switches PascRust
  fakes, and AssignCrt. Self-checking. (Free Pascal's Unix Crt differs from
  TP 7 here: its TextMode doesn't set LastMode, CheckBreak starts False, and
  AssignCrt can't read; so this .out is built from the checks, by hand.) }
uses Crt;
var
  checks, failures: Integer;
  f: Text;
  n: Integer;

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
  Check(LastMode = CO80, 'LastMode starts as CO80');
  TextMode(BW80);
  Check(LastMode = BW80, 'TextMode sets LastMode');
  LastMode := CO80;
  TextMode(LastMode);
  Check(LastMode = CO80, 'TextMode(LastMode)');
  TextColor(Yellow);
  TextBackground(Blue);
  Check(TextAttr = Yellow + Blue * 16, 'TextAttr from TextColor and TextBackground');
  TextAttr := White + Red * 16;
  Check(TextAttr = $4F, 'TextAttr :=');
  NormVideo;
  Check(TextAttr = LightGray, 'NormVideo resets TextAttr');
  Check(CheckBreak, 'CheckBreak starts True');
  CheckBreak := False;
  Check(not CheckBreak, 'CheckBreak :=');
  DirectVideo := False;
  CheckSnow := False;
  Check(not DirectVideo and not CheckSnow, 'DirectVideo and CheckSnow can be set');
  AssignCrt(f);
  Rewrite(f);
  WriteLn(f, 'written through AssignCrt');
  Close(f);
  Reset(f);
  ReadLn(f, n);
  Close(f);
  Check(n = 42, 'read through AssignCrt');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
