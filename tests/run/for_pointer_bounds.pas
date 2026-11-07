program ForPointerBounds;
{ A for loop's bounds that read through a pointer, with a body that
  changes the record the pointer points to. Pascal works out the bounds
  once, before the loop. }
type
   TList = record
      Count: Integer;
      Items: array[1..5] of Integer;
   end;
   PList = ^TList;
var
   P: PList;
   I: Integer;
begin
   New(P);
   P^.Count := 3;
   for I := 1 to P^.Count do
   begin
      P^.Items[I] := I * 2;
      P^.Count := 1;
   end;
   WriteLn(P^.Items[3], ' ', P^.Count);
   for I := Low(P^.Items) to High(P^.Items) do
      P^.Items[I] := I;
   WriteLn(P^.Items[5]);
   P^.Count := 4;
   for I := P^.Count downto 1 do
      Inc(P^.Items[I], 10);
   WriteLn(P^.Items[1], ' ', P^.Items[4], ' ', P^.Items[5]);
   Dispose(P);
end.
