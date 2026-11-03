{ Inc and Dec steps: read from the heap record being changed, on a
  Boolean (the enum (False, True)), numbers that need no cast, and ones
  that wrap or are negative. }
program IncDecSteps;
type
   Day = (Mon, Tue, Wed, Thu);
   PRec = ^TRec;
   TRec = record
      v, n: Integer;
      d: Day;
   end;
var
   p: PRec;
   b: Boolean;
   c: Char;
   w: Word;
   bt: Byte;
   i: Integer;
   sh: ShortInt;
begin
   New(p);
   p^.v := 42;
   p^.n := 2;
   Inc(p^.v, p^.n);
   p^.d := Mon;
   Inc(p^.d, p^.n);
   WriteLn(p^.v, ' ', Ord(p^.d));
   b := False;
   Inc(b);
   WriteLn(b);
   Dec(b);
   WriteLn(b);
   c := 'e';
   Dec(c, 2);
   Inc(c);
   w := 10;
   Inc(w, 300);
   WriteLn(c, ' ', w);
   bt := 1;
   Inc(bt, 300);
   Dec(bt, -1);
   i := 5;
   Inc(i, -2);
   Dec(i, 70000);
   sh := 10;
   Inc(sh, 200);
   WriteLn(bt, ' ', i, ' ', sh);
end.
