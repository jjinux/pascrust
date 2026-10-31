{ Standard routines that change a heap record while reading it: a value
  read from the record is worked out first, and Val's two targets in one
  record borrow it once. }
program HeapBuiltins;
type
   PRec = ^TRec;
   TRec = record
      name: string[10];
      w: Integer;
      buf: array[0..2] of Byte;
      v: Integer;
      code: Integer;
   end;
var
   p: PRec;
   s: string;
begin
   New(p);
   p^.name := '12';
   p^.w := 1;
   Insert(p^.name, p^.name, 1);
   WriteLn(p^.name);
   Delete(p^.name, p^.w, 1);
   WriteLn(p^.name);
   p^.w := 2;
   FillChar(p^.buf, p^.w, 65);
   WriteLn(p^.buf[0], ' ', p^.buf[1], ' ', p^.buf[2]);
   p^.v := 1;
   Move(p^.v, p^.buf, p^.w);
   WriteLn(p^.buf[0], ' ', p^.buf[1]);
   s := '12';
   Val(s, p^.v, p^.code);
   WriteLn(p^.v, ' ', p^.code);
   p^.name := '34';
   Val(p^.name, p^.v, p^.code);
   WriteLn(p^.v, ' ', p^.code);
end.
