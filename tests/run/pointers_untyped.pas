program PointersUntyped;
{ The untyped Pointer: holding a typed pointer and casting it back,
  comparisons with typed pointers and nil, a list of Pointers, GetMem for a
  typed pointer (the dynamic-array idiom) and for raw bytes read through
  p^, FreeMem. Output from fpc -Mtp. }
type
   PNode = ^Node;
   Node = record
      x: Integer;
   end;
   PInts = ^Ints;
   Ints = array[0..999] of Integer;
var
   p, q: Pointer;
   n, m: PNode;
   a: PInts;
   list: array[1..3] of Pointer;
   f: file;
   i: Integer;
begin
   New(n);
   n^.x := 5;
   p := n;
   WriteLn(PNode(p)^.x, ' ', p = n, ' ', Assigned(p));
   m := p;
   m^.x := 6;
   WriteLn(n^.x);
   list[1] := n;
   list[2] := nil;
   q := p;
   WriteLn(q = p, ' ', p = list[1]);
   WriteLn(list[1] = p, ' ', list[2] = nil);
   GetMem(a, 10 * SizeOf(Integer));
   for i := 0 to 9 do a^[i] := i * i;
   WriteLn(a^[3]);
   FreeMem(a, 10 * SizeOf(Integer));
   GetMem(q, 4);
   Assign(f, 'p.bin');
   Rewrite(f, 1);
   i := 513;
   BlockWrite(f, i, 2);
   Close(f);
   Reset(f, 1);
   BlockRead(f, q^, 2);
   Close(f);
   Erase(f);
   Move(q^, i, 2);
   WriteLn(i);
   FreeMem(q, 4);
   WriteLn(q <> nil);
end.
