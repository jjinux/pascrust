{ A pointer type written in place, not named, in a record field (a
  linked list's Next: ^Node), an array type's elements and a variant
  part. }
program AnonymousPointerFields;
type
   Node = record
      Value: Integer;
      Next: ^Node;
   end;
   Slots = array[1..2] of ^Node;
   Either = record
      case Boolean of
         False: (N: ^Node);
         True: (Count: Integer);
   end;
var
   s: Slots;
   e: Either;
   p: ^Node;
begin
   New(s[1]);
   s[1]^.Value := 7;
   New(s[1]^.Next);
   s[1]^.Next^.Value := 8;
   s[1]^.Next^.Next := nil;
   e.N := s[1];
   p := e.N;
   while p <> nil do
   begin
      WriteLn(p^.Value);
      p := p^.Next;
   end;
end.
