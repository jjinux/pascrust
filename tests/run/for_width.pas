program ForWidth;
{ A loop variable declared as Integer is 16 bits even when both bounds are
  literals, so i * 3000 wraps as it does in Turbo Pascal. (Free Pascal
  computes in 32 bits here, so this test's .out is written by hand.) }
var
   i: integer;
begin
   for i := 10 to 12 do
      writeln(i * 3000);
   for i := 12 downto 11 do
      writeln(i * 3000);
end.
