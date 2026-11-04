{ A unit Counter uses, with variables of its own (one named as one of
  Counter's) and an initialization part. }
unit Helper;
interface
   var
      HelperCount: Integer;
      Count: Integer;      { same name as Counter's }
   procedure Tick;
   function HelperName: string;
implementation
   procedure Tick;
      begin
         Inc(HelperCount);
         Inc(Count);
      end;
   function HelperName: string;
      begin
         HelperName := 'helper';
      end;
begin
   HelperCount := 1;
end.
