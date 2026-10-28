program Input;
var
  name: string;
  a, b: integer;
  c: char;
begin
  write('Your name? ');
  readln(name);
  write('Two numbers? ');
  readln(a, b);
  read(c);
  readln;
  writeln('Hello, ', name, '! ', a, ' + ', b, ' = ', a + b, '; char ', c);
end.
