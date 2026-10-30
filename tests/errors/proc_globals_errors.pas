program ProcGlobalsErrors;
{ A global passed by var through a procedural value whose routines also
  use it: two references to one variable in Rust. }
type Bump = procedure(var N: Integer);
var Count: Integer; P: Bump;
procedure AddCount(var N: Integer); begin Inc(Count); N := N + 1; end;
begin P := AddCount; P(Count); end.
