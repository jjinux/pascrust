program PointerParameter;
{ A parameter's type must be a type's name in TP ("Type identifier
  expected"), so a pointer type needs one: type PNode = ^Node. }
type
   Node = record
      V: Integer;
   end;

procedure Show(p: ^Node);
   begin
      WriteLn(p^.V);
   end;

begin
end.
