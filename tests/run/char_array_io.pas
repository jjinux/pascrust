program CharArrayIO;
{ TP 7's extended syntax for C-style strings in the standard routines:
  Write and Read of a zero-based Char array (up to its #0, and filled with
  one), Str into one, Val from one, Rename with a PChar. Output from
  fpc -Mtp, except the last two lines: Free Pascal doesn't take Val of a
  PChar or Assign with a Char array, which the TP 7 Language Guide allows
  (chapter 16), so those are by hand. }
{$X+}
uses Strings;
var
   a, b: array[0..15] of Char;
   p: PChar;
   n, code: Integer;
   r: Real;
   f: Text;
begin
   StrCopy(a, 'hi');
   Write(a); WriteLn('|');
   WriteLn(a, '|', StrLen(a), '|', a:5, '|');
   Str(42:5, b);
   WriteLn('[', b, '] ', StrLen(b));
   Str(3.5:6:2, b);
   WriteLn('[', b, ']');
   StrCopy(b, '456');
   Val(b, n, code);
   WriteLn(n, ' ', code);
   StrCopy(b, '2.5');
   Val(b, r, code);
   WriteLn(r:0:2, ' ', code);
   Assign(f, 'xs_tmp.txt');
   Rewrite(f);
   WriteLn(f, 'line one is longer than sixteen');
   WriteLn(f, '7');
   Close(f);
   p := 'xs_tmp2.txt';
   Rename(f, p);
   Assign(f, 'xs_tmp2.txt');
   Reset(f);
   ReadLn(f, b);
   ReadLn(f, a);
   Close(f);
   WriteLn('[', b, '] ', StrLen(b), ' [', a, ']');
   Erase(f);
   p := '78x';
   Val(p, n, code);
   WriteLn(n, ' ', code);
   StrCopy(a, 'xs_tmp3.txt');
   Assign(f, a);
   Rewrite(f);
   Close(f);
   Erase(f);
   WriteLn('assigned');
end.
