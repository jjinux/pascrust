{ Review batch K: a function ending in repeat ... until False (with and
  without Continue); a goto inside, out of and back to a while True loop;
  a labeled Break; and with p^ changing p. No unreachable code. }
program EndlessLoops2;
label
   L, Done;
type
   PRec = ^TRec;
   TRec = record
      x: Integer;
   end;
var
   k, n: Integer;
   p, q: PRec;

function Count: Integer;
   begin
      repeat
         Inc(k);
         if k = 3 then
         begin
            Count := k;
            Exit;
         end;
      until False;
   end;

function CountOdd: Integer;
   begin
      repeat
         Inc(k);
         if not Odd(k) then
            Continue;
         if k > 6 then
         begin
            CountOdd := k;
            Exit;
         end;
      until False;
   end;

function Inside: Integer;
   label
      Back;
   begin
      n := 0;
      while True do
      begin
         Back:
         Inc(n);
         if n < 3 then
            goto Back;
         if n > 5 then
         begin
            Inside := n;
            Exit;
         end;
      end;
      WriteLn('never');
   end;

function BackTo: Integer;
   label
      Top;
   begin
      n := 0;
      Top:
      Inc(n);
      while True do
      begin
         if n < 3 then
            goto Top;
         BackTo := n;
         Exit;
      end;
   end;

begin
   k := 0;
   WriteLn(Count, ' ', CountOdd, ' ', Inside, ' ', BackTo);
   k := 0;
   while True do
   begin
      Inc(k);
      if k = 3 then
         goto Done;
   end;
   WriteLn('never');
   Done:
   WriteLn('done ', k);
   k := 0;
   while True do
   begin
      Inc(k);
      if k = 2 then
         L: Break;
   end;
   WriteLn('labeled break ', k);
   New(p);
   New(q);
   q^.x := 7;
   with p^ do
      p := q;
   WriteLn(p^.x);
end.
