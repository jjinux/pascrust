program GraphRefused;
{ Graph routines that load or link driver code PascRust doesn't have. }
uses Graph;
var
  n: Integer;
begin
  n := InstallUserDriver('SVGA256', nil);
  n := RegisterBGIfont(nil);
end.
