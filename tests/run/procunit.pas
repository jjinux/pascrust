{ A unit with a procedural type; the program keeps its own routines in
  it (see proc_unit_types.pas). }
unit ProcUnit;
interface
type
  Handler = procedure(N: Integer);
var
  OnEvent: Handler;
procedure Fire(N: Integer);
implementation
procedure Fire(N: Integer);
begin
  if Assigned(OnEvent) then
    OnEvent(N);
end;
end.
