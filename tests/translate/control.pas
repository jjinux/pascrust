program Control;
var
  i, total: integer;
  c: char;
  done: boolean;
begin
  total := 0;
  for i := 1 to 10 do
    total := total + i;
  writeln('sum 1..10 = ', total);

  for i := 3 downto 1 do
    write(i, ' ');
  writeln;

  i := 0;
  while i < 5 do
    i := i + 2;
  writeln('while ended at ', i);

  i := 10;
  repeat
    i := i - 3
  until i < 0;
  writeln('repeat ended at ', i);

  for c := 'a' to 'e' do
    write(c);
  writeln;

  for i := 1 to 5 do
    case i of
      1: writeln('one');
      2, 3: writeln('two or three');
    else
      writeln('big')
    end;

  done := false;
  if not done and (total > 50) then
    writeln('yes')
  else if done then
    writeln('done')
  else
    writeln('no');

  for i := 1 to 100 do
  begin
    if i mod 7 = 0 then
      continue;
    if i > 10 then
      break;
    write(i, ' ');
  end;
  writeln;
end.
