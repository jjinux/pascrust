program CrtPlain;
{ Crt in plain mode (output is not a terminal): cursor and colour calls do
  nothing, and ReadKey reads characters from standard input.
  crt_plain.out is hand-written: Free Pascal's Crt emits escape codes even
  when its output is piped. }
uses Crt;
var
  c: char;
  n: integer;
begin
  ClrScr;
  TextColor(Yellow);
  TextBackground(Blue);
  GotoXY(10, 5);
  writeln('Press keys...');
  n := 0;
  repeat
    c := ReadKey;
    n := n + 1;
    writeln('key ', n, ': ', ord(c));
  until upcase(c) = 'Q';
  if KeyPressed then writeln('more keys waiting');
  writeln('window ', lo(WindMax) + 1, 'x', hi(WindMax) + 1);
  Delay(1000);
  NormVideo;
  writeln('bye');
end.
