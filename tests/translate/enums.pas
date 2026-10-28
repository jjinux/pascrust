program Enums;
type
  Choice = (Paper, Scissors, Rock);
  Outcome = (Win, Lose, Tie);
var
  a, b: Choice;
  results: array[Outcome] of integer;
  o: Outcome;

function Play(mine, theirs: Choice): Outcome;
begin
  if mine = theirs then
    Play := Tie
  else if ((mine = Paper) and (theirs = Rock)) or
          ((mine = Scissors) and (theirs = Paper)) or
          ((mine = Rock) and (theirs = Scissors)) then
    Play := Win
  else
    Play := Lose;
end;

procedure Show(c: Choice);
begin
  case c of
    Paper: write('paper');
    Scissors: write('scissors');
    Rock: write('rock');
  end;
end;

begin
  for o := Win to Tie do
    results[o] := 0;
  for a := Paper to Rock do
    for b := Paper to Rock do
      results[Play(a, b)] := results[Play(a, b)] + 1;
  writeln(results[Win], ' ', results[Lose], ' ', results[Tie]);
  a := Paper;
  a := succ(a);
  Show(a);
  writeln(' ', ord(a), ' ', ord(Rock));
  b := Choice(2);
  Show(pred(b));
  writeln;
  if Rock > Paper then writeln('ordered');
end.
