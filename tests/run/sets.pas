program Sets;
{ set of T: literals with ranges, in, + - *, comparisons, Include and
  Exclude, set constants, and sets in records and parameters. }
type
  Color = (Red, Green, Blue, Yellow);
  Colors = set of Color;
  CharSet = set of Char;
  Digit = 0..9;
  Digits = set of Digit;
  Palette = record
    name: string[10];
    colors: Colors;
  end;

const
  Vowels = ['a', 'e', 'i', 'o', 'u'];
  Primary = [Red, Green, Blue];
  Typed: CharSet = ['x'..'z'];
  Warmish: Colors = [Red..Green];

var
  letters, found: CharSet;
  c: Char;
  d: Digits;
  i: Integer;
  p: Palette;
  warm: Colors;
  hue, other: Color;
  flags: set of Boolean;
  charSets: array[1..2] of CharSet;

{ Reads a global, which Rust passes in: `s[i] := s[i] + Second` can't
  borrow it twice. }
function Second: CharSet;
begin
  Second := charSets[2];
end;

procedure Show(s: CharSet);
var
  ch: Char;
begin
  Write('[');
  for ch := 'a' to 'z' do
    if ch in s then
      Write(ch);
  WriteLn(']');
end;

procedure AddDigit(var s: Digits; n: Integer);
begin
  if n in [0..9] then
    Include(s, n);
end;

function Count(s: Digits): Integer;
var
  n, k: Integer;
begin
  k := 0;
  for n := 0 to 9 do
    if n in s then
      k := k + 1;
  Count := k;
end;

begin
  letters := ['a'..'f', 'x', 'q'];
  Show(letters);
  Show(letters * Vowels);
  Show(letters - Vowels);
  Show(letters + ['m'..'o']);
  Show(Typed);
  found := [];
  for c := 'a' to 'z' do
    if (c in Vowels) and not (c in ['i', 'o']) then
      Include(found, c);
  Show(found);
  Exclude(found, 'a');
  Show(found);
  WriteLn(found <= Vowels, ' ', Vowels <= found, ' ', Vowels >= found, ' ', found = ['e', 'u'], ' ', found <> []);
  c := 'k';
  WriteLn(c in ['a'..'m'], ' ', c in letters, ' ', 'e' in letters);
  d := [];
  for i := 1 to 12 do
    if i mod 3 = 0 then
      AddDigit(d, i);
  WriteLn(Count(d), ' ', 3 in d, ' ', 12 in d, ' ', 4 in d);
  p.name := 'flag';
  p.colors := Primary - [Green];
  warm := [Red, Yellow];
  WriteLn(Red in p.colors, ' ', Green in p.colors, ' ', p.colors * warm = [Red], ' ', Yellow in Primary);
  i := 0;
  for c := 'a' to 'z' do
    if c in Vowels + ['y'] then
      i := i + 1;
  WriteLn(i);
  { s := s + [x], s := s * [...], s := s - [x] }
  letters := ['a'];
  letters := letters + ['b'];
  letters := letters + ['c'..'e'];
  letters := letters - ['a'];
  letters := letters * ['b'..'d'];
  Show(letters);
  { enum and Boolean ranges in set values and tests }
  warm := [Red..Blue];
  WriteLn(Blue in warm, ' ', Yellow in warm, ' ', Green in Warmish);
  flags := [False..True];
  WriteLn(False in flags, ' ', True in flags);
  hue := Blue;
  other := Yellow;
  WriteLn(hue in [Red..Green, other], ' ', Yellow in [Red..Green, other]);
  { one element of a set array from another }
  charSets[1] := ['a'];
  charSets[2] := ['b'];
  i := 2;
  charSets[i] := charSets[i] + charSets[1];
  Show(charSets[2]);
  charSets[1] := charSets[1] + Second;
  Show(charSets[1]);
  { backwards ranges are empty }
  WriteLn(hue in [Yellow..Red], ' ', True in [True..False], ' ', i in [5..1, 2], ' ', 'm' in ['z'..'a']);
end.
