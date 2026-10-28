program Strings;
var
  s, t: string;
  short: string[5];
  c: char;
  n, code: integer;
  r: real;
begin
  s := 'Turbo';
  t := s + ' Pascal';
  writeln(t, ' has ', length(t), ' characters');
  writeln(copy(t, 7, 6));
  writeln(pos('Pas', t));
  delete(t, 1, 6);
  writeln(t);
  insert('Free ', t, 1);
  writeln(t);
  short := 'abcdefgh';
  writeln(short);
  c := t[1];
  writeln(c, ' ', upcase('q'), ' ', ord(c), ' ', chr(65));
  t[1] := 'f';
  writeln(t);
  if s < 'Zebra' then writeln('Turbo < Zebra');
  if s = 'Turbo' then writeln('equal');
  str(42, s);
  writeln('[', s, ']');
  str(3.14159:6:2, s);
  writeln('[', s, ']');
  val('123', n, code);
  writeln(n, ' ', code);
  val('12x', n, code);
  writeln(code);
  writeln('it''s', ' ', 'a':3, ' ', 'end'#33);
  writeln(concat('a', 'b', 'c'));
  r := 2.5;
  writeln(r:8:3, '|', 7:4, '|', true:6);
end.
