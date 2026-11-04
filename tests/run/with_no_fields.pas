{ A with whose body uses no field (`with r do goto Done`) needs no comment
  about writing fields out; one that does gets it. }
program WithNoFields;
label Done;
type TR = record x: Integer; end;
var r: TR;
begin
   with r do
      goto Done;
   WriteLn(1);
   Done:
   with r do
      x := 2;
   WriteLn(r.x);
end.
