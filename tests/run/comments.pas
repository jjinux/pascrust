program Comments;
{ Pascal comments are carried into the Rust: this header, comments at
  the ends of lines, comments before statements and declarations, and
  comments before an end. }
const
  Limit = 3;   { how many rounds }
type
  Point = record
    x, y: Integer;   { position }
    (* the point's name *)
    name: string[8];
  end;
var
  p: Point;    // where we are
  i, total: Integer;

{ Add n to the total, twice. }
procedure AddTwice(n: Integer);
begin
  total := total + n;   { once }
  total := total + n    { twice }
  { nothing else to do }
end;

begin
  total := 0;
  { count up }
  for i := 1 to Limit do
    AddTwice(i);   // 2, 4, 6
  p.x := total;
  p.name := 'here';
  WriteLn(p.name, ' ', p.x);
  { done }
end.
