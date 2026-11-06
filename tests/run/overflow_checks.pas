{ The $Q+ switch: integer +, -, *, negation, Abs, Sqr, Succ, Pred, Inc
  and Dec stop with runtime error 215 when the result doesn't fit, as in
  TP 7; code compiled with $Q- wraps around. Expected output by hand: Free Pascal works in
  32 or 64 bits, so 32767 + 1 doesn't overflow there. }
program OverflowChecks;
var
   i, j: Integer;
   b: Byte;
   l: LongInt;

{$Q+}
procedure Bump(var n: Integer; by: Integer);
   begin
      Inc(n, by);
   end;
{$Q-}

begin
   i := 32767;
   j := i + 1;
   WriteLn(j);
   {$Q+}
   l := 100000;
   WriteLn(i - 1, ' ', l * 2, ' ', -i, ' ', Abs(j + 1), ' ', Succ(i - 1), ' ', Pred(j + 1));
   b := 254;
   Inc(b);
   Dec(b, 255);
   j := 181;
   Bump(j, 0);
   WriteLn(b, ' ', Sqr(j), ' ', 2 * j);
   {$Q-}
   WriteLn(i + 1);
   {$Q+}
   i := i + 1;
   WriteLn('not reached');
end.
