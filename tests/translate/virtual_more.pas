program VirtualMore;
{ Virtual methods with parameters and results, one first declared in a
  descendant, an object with no methods of its own, and objects in a
  record. (Turbo Pascal needs a constructor call before any virtual call.) }
type
  TAnimal = object
    name: string[10];
    constructor Init(aname: string);
    function Sound: string; virtual;
    function Speak(times: Integer): string; virtual;
  end;
  PDog = ^TDog;
  TDog = object(TAnimal)
    function Sound: string; virtual;
    function Tricks: Integer; virtual;
    procedure Report;
  end;
  TPuppy = object(TDog)
    function Tricks: Integer; virtual;
  end;
  TOldDog = object(TDog)
  end;
  Pair = record
    first: TDog;
    second: TPuppy;
  end;

var
  pets: Pair;
  d: PDog;
  old: TOldDog;

constructor TAnimal.Init(aname: string);
begin
  name := aname;
end;

function TAnimal.Sound: string;
begin
  Sound := '...';
end;

function TAnimal.Speak(times: Integer): string;
var
  s: string;
  i: Integer;
begin
  s := name + ':';
  for i := 1 to times do
    s := s + ' ' + Sound;
  Speak := s;
end;

function TDog.Sound: string;
begin
  Sound := 'woof';
end;

function TDog.Tricks: Integer;
begin
  Tricks := 1;
end;

procedure TDog.Report;
begin
  WriteLn(Speak(2), ' (', Tricks, ' tricks)');
end;

function TPuppy.Tricks: Integer;
begin
  Tricks := inherited Tricks + 2;
end;

begin
  pets.first.Init('Rex');
  pets.second.Init('Bit');
  pets.first.Report;
  pets.second.Report;
  old.Init('Max');
  old.Report;
  New(d, Init('Ace'));
  WriteLn(d^.Tricks, ' ', d^.Speak(1));
  Dispose(d);
end.
