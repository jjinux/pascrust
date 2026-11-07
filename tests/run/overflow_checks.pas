{ The $Q+ switch: integer +, -, *, negation, Abs, Sqr, Succ, Pred, Inc
  and Dec stop with runtime error 215 when the result doesn't fit, as in
  TP 7; code compiled with $Q- wraps around. Expected output by hand:
  Free Pascal works in 32 or 64 bits, so 32767 + 1 doesn't overflow
  there. }
program OverflowChecks;
const
   K = 5;
type
   Rec = record
      N: Integer;
   end;
   PRec = ^Rec;
   Outer = record
      q: PRec;
   end;
var
   p: ^Rec;
   r: ^Outer;
   i, j: Integer;
   b: Byte;
   l: LongInt;
   w: Word;
   s: ShortInt;

{$Q+}
procedure Bump(var N: Integer; by: Integer);
   begin
      Inc(N, by);
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
   New(p);
   p^.N := 1;
   Inc(p^.N);
   with p^ do
      Inc(N, 2);
   WriteLn(p^.N, ' ', (2 + 3) * j);
   New(r);
   r^.q := p;
   Inc(r^.q^.N, 40);
   WriteLn(p^.N);
   { A constant (2 + 3) is worked out when TP compiles, so it isn't checked. }
   b := 10;
   Dec(b, 2 + 3);
   w := 10;
   w := w * (2 + 3);
   WriteLn(j * (2 + 3), ' ', (2 + 3) * (4 + 5), ' ', l - (2 + 3), ' ', b, ' ', w);
   { A constant step that's negative turns round, as -5 does: up by 5. }
   b := 250;
   Dec(b, -K);
   Inc(p^.N, K);
   WriteLn(b, ' ', p^.N);
   { A variable step that's negative adds as a number: 10 + -5 is 5. }
   b := 10;
   s := -5;
   Inc(b, s);
   Dec(r^.q^.N, s);
   w := 3;
   Inc(w, s + 2);
   WriteLn(b, ' ', p^.N, ' ', w);
   {$Q-}
   WriteLn(i + 1);
   {$Q+}
   i := i + 1;
   WriteLn('not reached');
end.
