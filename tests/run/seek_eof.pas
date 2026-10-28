program SeekEofTest;
{ SeekEof and SeekEoln skip blanks first, so a loop reading numbers stops
  cleanly at trailing blanks and empty lines. }
var
  n, sum, count: Integer;
begin
  sum := 0;
  count := 0;
  while not SeekEof do
  begin
    while not SeekEoln do
    begin
      Read(n);
      sum := sum + n;
      count := count + 1;
    end;
    ReadLn;
    WriteLn('line done, ', count, ' numbers so far');
  end;
  WriteLn(count, ' numbers, sum ', sum);
end.
