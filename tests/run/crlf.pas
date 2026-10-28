program CrLf;
{ This file has DOS (CR LF) line endings, and so does its input; the
  program also reads back a text file written with them. }
var
  f: text;
  name, line: string;
  n: integer;
begin
  readln(name);
  readln(n);
  writeln('name=[', name, '] n=', n);
  assign(f, 'crlf.txt');
  rewrite(f);
  write(f, 'first', #13#10, 'second', #13#10);
  close(f);
  reset(f);
  while not eof(f) do
  begin
    readln(f, line);
    writeln('[', line, '] length ', length(line));
  end;
  close(f);
  erase(f);
end.
