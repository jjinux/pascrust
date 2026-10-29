program OverlayTest;
{ The Overlay unit, faked: there's nothing to load, so it reports success
  and remembers its settings. Self-checking. (Free Pascal has no overlay
  manager, so this .out is built from the checks, by hand.) }
{$F+}
uses Overlay, OvlHelp;
{$O OvlHelp}
var
  checks, failures: Integer;

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
  OvrInit('OVERLAY.OVR');
  Check(OvrResult = ovrOk, 'OvrInit succeeds without a .OVR file');
  OvrInitEMS;
  Check(OvrResult = ovrOk, 'OvrInitEMS succeeds');
  OvrSetBuf(OvrGetBuf + 16384);
  Check(OvrGetBuf = 65536 + 16384, 'OvrSetBuf and OvrGetBuf');
  OvrSetRetry(OvrGetBuf div 3);
  Check(OvrGetRetry = (65536 + 16384) div 3, 'OvrSetRetry and OvrGetRetry');
  OvrClearBuf;
  Check(Twice(21) = 42, 'calling into an overlaid unit');
  Check((OvrLoadCount = 0) and (OvrTrapCount = 0), 'nothing is ever loaded');
  OvrResult := ovrError;
  Check(OvrResult = -1, 'OvrResult can be set');
  OvrFileMode := 0;
  Check(OvrFileMode = 0, 'OvrFileMode can be set');
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.
