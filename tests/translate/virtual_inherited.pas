program VirtualInherited;
{ Each object's constructor and destructor calls its parent's with
  inherited, three levels deep, on plain variables and through pointers. }
type
  PBase = ^TBase;
  TBase = object
    id: Integer;
    log: string;
    constructor Init(anId: Integer);
    destructor Done; virtual;
    procedure Describe; virtual;
  end;
  PMiddle = ^TMiddle;
  TMiddle = object(TBase)
    size: Integer;
    constructor Init(anId, aSize: Integer);
    destructor Done; virtual;
    procedure Describe; virtual;
  end;
  PLeaf = ^TLeaf;
  TLeaf = object(TMiddle)
    color: string[8];
    constructor Init(anId, aSize: Integer; aColor: string);
    destructor Done; virtual;
    procedure Describe; virtual;
  end;

var
  b: TBase;
  m: TMiddle;
  l: TLeaf;
  p: PBase;

constructor TBase.Init(anId: Integer);
begin
  id := anId;
  log := 'base';
end;

destructor TBase.Done;
begin
  WriteLn('  base done ', id);
end;

procedure TBase.Describe;
begin
  WriteLn('base ', id, ' (', log, ')');
end;

constructor TMiddle.Init(anId, aSize: Integer);
begin
  inherited Init(anId);
  size := aSize;
  log := log + ' middle';
end;

destructor TMiddle.Done;
begin
  WriteLn('  middle done ', size);
  inherited Done;
end;

procedure TMiddle.Describe;
begin
  Write('middle of size ', size, ': ');
  inherited Describe;
end;

constructor TLeaf.Init(anId, aSize: Integer; aColor: string);
begin
  inherited Init(anId, aSize);
  color := aColor;
  log := log + ' leaf';
end;

destructor TLeaf.Done;
begin
  WriteLn('  leaf done ', color);
  inherited Done;
end;

procedure TLeaf.Describe;
begin
  Write(color, ' leaf, ');
  inherited Describe;
end;

begin
  b.Init(1);
  m.Init(2, 20);
  l.Init(3, 30, 'red');
  b.Describe;
  m.Describe;
  l.Describe;
  l.Done;
  p := New(PLeaf, Init(4, 40, 'blue'));
  p^.Describe;
  Dispose(p, Done);
  New(p, Init(5));
  p^.Describe;
  Dispose(p, Done);
end.
