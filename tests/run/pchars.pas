program PChars;
{ PChar and the Strings unit: string literals and zero-based Char arrays
  used as PChars, indexing, arithmetic, comparisons, Inc/Dec, GetMem,
  StrNew/StrDispose, and a Char array written through its PChar in a
  routine, a nested call, while and if conditions, for bounds and a case
  selector. Output from fpc -Mtp. }
{$X+}
uses Strings;
type
   TName = array[0..19] of Char;
var
   buf: array[0..79] of Char;
   name: TName;
   p, q, h: PChar;
   n: Word;
   i: Integer;

procedure Show(s: PChar);
   begin
      WriteLn('[', s, '] ', StrLen(s));
   end;

function CountChar(s: PChar; c: Char): Integer;
   var
      k: Integer;
   begin
      k := 0;
      while s^ <> #0 do
      begin
         if s^ = c then
            Inc(k);
         Inc(s);
      end;
      CountChar := k;
   end;

procedure Shout(s: PChar);
   begin
      StrUpper(s);
   end;

begin
   p := 'Hello';
   WriteLn(p, ' ', StrLen(p), ' ', p[1], ' ', p^);
   StrPCopy(buf, 'hello, world');
   WriteLn(buf[0], buf[5], ' ', StrLen(buf));
   Show(buf);
   StrCat(buf, '!');
   Show(buf);
   WriteLn(StrPas(StrScan(buf, ',')));
   WriteLn(StrEnd(buf) - buf);
   WriteLn(StrPos(buf, 'world') - buf);
   WriteLn(CountChar(buf, 'o'));
   Shout(buf);
   WriteLn(StrPas(buf));
   StrCopy(name, 'Ada');
   StrLCat(name, ' Lovelace and more', 10);
   WriteLn('[', StrPas(name), ']');
   WriteLn(StrComp('abc', 'abd') < 0, ' ', StrIComp('ABC', 'abc'), ' ', StrLComp('abcx', 'abcy', 3));
   h := StrNew(buf);
   q := h + 7;
   WriteLn(q);
   q[0] := 'w';
   Dec(q, 2);
   WriteLn(q, ' ', q - h, ' ', q > h, ' ', q = h + 5);
   StrDispose(h);
   GetMem(p, 10);
   StrCopy(p, 'abc');
   i := 0;
   while p[i] <> #0 do
   begin
      p[i] := UpCase(p[i]);
      Inc(i);
   end;
   WriteLn(p, ' ', Assigned(p), ' ', StrRScan('a.b.c', '.') - StrScan('a.b.c', '.'));
   FreeMem(p, 10);
   p := nil;
   WriteLn(p = nil, ' ', StrPas(StrNew('')) = '');
   StrLower(StrCopy(buf, 'MiXeD'));
   WriteLn(buf[0], buf[1], buf[2]);
   n := StrLen(StrECopy(StrECopy(buf, 'one'), 'two'));
   WriteLn(StrPas(buf), ' ', n);
   StrCopy(buf, '');
   while StrLen(StrCat(buf, 'ab')) < 7 do
      Write('.');
   WriteLn(' ', StrPas(buf));
   if StrComp(StrUpper(buf), 'ABABABAB') = 0 then
      WriteLn('upper ', StrPas(buf))
   else
      WriteLn('not ', StrPas(buf));
   StrCopy(buf, 'ab');
   for i := 1 to StrLen(StrCat(buf, 'cd')) do
      Write(i);
   WriteLn(' ', StrPas(buf));
   case StrLen(StrCat(buf, 'e')) of
      5: WriteLn('five ', StrPas(buf));
      else
         WriteLn('other');
   end;
end.
