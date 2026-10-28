program Records;
type
  Person = record
    name: string[10];
    age: integer;
  end;
  Point = record
    x, y: real;
  end;
var
  people: array[1..3] of Person;
  p, q: Person;
  origin: Point;
  i: integer;

procedure Birthday(var who: Person);
begin
  who.age := who.age + 1;
end;

function Describe(who: Person): string;
var
  s: string;
begin
  str(who.age, s);
  Describe := who.name + ' (' + s + ')';
end;

begin
  p.name := 'Alexander the Great';
  p.age := 32;
  writeln(p.name, ' ', length(p.name));
  q := p;
  q.age := 1;
  Birthday(p);
  writeln(Describe(p), ' ', Describe(q));
  for i := 1 to 3 do
  begin
    people[i].name := 'P';
    people[i].age := i * 10;
  end;
  Birthday(people[2]);
  writeln(people[2].age);
  origin.x := 1.5;
  writeln(origin.x:0:1, ' ', origin.y:0:1);
end.
