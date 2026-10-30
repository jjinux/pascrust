program ProcGlobals;
{ Routines kept in a procedural variable that use globals: every routine
  of the type takes the globals any of them uses, and so does a call
  through one (Run). Output from fpc -Mtp. }
type
   Bump = procedure(var N: Integer);
var
   Count, Other: Integer;
   P: Bump;
procedure AddOne(var N: Integer); begin Inc(N); end;
procedure AddCount(var N: Integer); begin Inc(Count); N := N + Count; end;
procedure Run(Q: Bump; times: Integer);
   var i: Integer;
   begin
      for i := 1 to times do Q(Other);
   end;
begin
   Count := 0; Other := 0;
   P := AddOne;
   P(Other);
   Run(AddCount, 3);
   WriteLn(Other, ' ', Count);
   P := AddCount;
   Run(P, 2);
   WriteLn(Other, ' ', Count);
end.
