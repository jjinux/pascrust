unit OvlHelp;
{ A unit that overlay_test.pas marks for overlaying. TP required units in
  overlays to be compiled with the O+ switch, as below. }
{$O+,F+}
interface

function Twice(n: Integer): Integer;

implementation

function Twice(n: Integer): Integer;
begin
  Twice := n * 2;
end;

end.
