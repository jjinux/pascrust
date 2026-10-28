program EofUnterminated;
{ The last line of a text file needn't end with a line break; Eof is true
  right after its last character. }
var
   f: text;
   c: char;
begin
   assign(f, 'eof_unterminated.tmp');
   rewrite(f);
   write(f, 'ab', 'cd');
   close(f);
   reset(f);
   while not eof(f) do
   begin
      read(f, c);
      write(ord(c), ' ');
   end;
   writeln;
   close(f);
   erase(f);
end.
