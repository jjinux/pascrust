program HiddenCaptures;
{ A nested procedure that needs two variables with the same name: its
  parent's local `count`, and the global `count` it hides, which a routine
  it calls uses. Each gets its own parameter. Output from fpc -Mtp. }
var
   count: Integer;

procedure Note;
   begin
      Inc(count);
   end;

procedure Hidden;
   var
      count: Integer;

   procedure Deeper;
      begin
         Note;
         Inc(count);
      end;

   begin
      count := 100;
      Deeper;
      WriteLn('hidden count ', count);
   end;

begin
   count := 0;
   Hidden;
   WriteLn('count ', count);
end.
