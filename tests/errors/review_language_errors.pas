program ReviewLanguageErrors;
{ New and Dispose name a constructor and a destructor; a with field can't
  be a for loop's variable; a pointer and a field behind it can't both be
  var arguments. }
type
  PShape = ^TShape;
  TShape = object
    x: Integer;
    constructor Init;
    destructor Done;
    procedure Draw;
  end;
  Counter = record
    i, total: Integer;
  end;
var
  p: PShape;
  c: Counter;

constructor TShape.Init; begin end;
destructor TShape.Done; begin end;
procedure TShape.Draw; begin end;

procedure PtrAndField(var q: PShape; var n: Integer);
begin
end;

begin
  New(p, Draw);
  Dispose(p, Init);
  with c do
    for i := 1 to 3 do
      total := total + i;
  PtrAndField(p, p^.x);
end.
