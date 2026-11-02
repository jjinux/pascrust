{ Inc and Dec on enum variables, elements, heap fields and var
  parameters, with and without a step. }
program EnumIncDec;
type
   Day = (Mon, Tue, Wed, Thu, Fri, Sat, Sun);
   PRec = ^TRec;
   TRec = record
      d: Day;
   end;
var
   d: Day;
   days: array[1..2] of Day;
   p: PRec;
   n: Integer;

procedure Bump(var x: Day);
   begin
      Inc(x);
   end;

begin
   d := Mon;
   Inc(d);
   WriteLn(Ord(d));
   Inc(d, 3);
   WriteLn(Ord(d));
   Dec(d);
   Dec(d, 2);
   WriteLn(Ord(d));
   days[2] := Sat;
   Inc(days[2]);
   WriteLn(Ord(days[2]));
   New(p);
   p^.d := Wed;
   Inc(p^.d);
   n := 2;
   Dec(p^.d, n);
   WriteLn(Ord(p^.d));
   Bump(d);
   WriteLn(Ord(d));
end.
