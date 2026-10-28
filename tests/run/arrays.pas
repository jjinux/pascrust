program Arrays;
const
  MaxCol = 5;
  MaxRow = 3;
  Days: array[1..3] of integer = (31, 28, 31);
type
  GridT = array[0..MaxCol, 0..MaxRow] of boolean;
  Counts = array['a'..'e'] of integer;
var
  grid: GridT;
  tally: Counts;
  squares: array[1..10] of longint;
  i, c, r, alive: integer;
  ch: char;
  word: string;

procedure Clear(var g: GridT);
var
  c, r: integer;
begin
  for c := 0 to MaxCol do
    for r := 0 to MaxRow do
      g[c, r] := false;
end;

function CountAlive(g: GridT): integer;
var
  c, r, n: integer;
begin
  n := 0;
  for c := 0 to MaxCol do
    for r := 0 to MaxRow do
      if g[c][r] then
        n := n + 1;
  CountAlive := n;
end;

begin
  for i := 1 to 10 do
    squares[i] := i * i;
  writeln(squares[1], ' ', squares[10]);
  writeln(Days[1] + Days[2] + Days[3]);

  Clear(grid);
  grid[1, 1] := true;
  grid[2, 3] := true;
  writeln('alive: ', CountAlive(grid));

  word := 'abacabad';
  for ch := 'a' to 'e' do
    tally[ch] := 0;
  for i := 1 to length(word) do
    if word[i] <= 'e' then
      tally[word[i]] := tally[word[i]] + 1;
  for ch := 'a' to 'e' do
    write(ch, '=', tally[ch], ' ');
  writeln;
end.
