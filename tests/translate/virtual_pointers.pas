program VirtualPointers;
{ Pointers to a base object type can point to descendants; a virtual
  method called through them runs the version for the real object. }
type
  PShape = ^TShape;
  TShape = object
    x: Integer;
    next: PShape;
    constructor Init(ax: Integer);
    procedure Draw; virtual;
    procedure Show;
    destructor Done; virtual;
  end;
  PCircle = ^TCircle;
  TCircle = object(TShape)
    radius: Integer;
    constructor Init(ax, r: Integer);
    procedure Draw; virtual;
    destructor Done; virtual;
  end;
  PSquare = ^TSquare;
  TSquare = object(TShape)
    side: Integer;
    procedure Draw; virtual;
  end;

var
  list, p: PShape;
  pc: PCircle;
  ps: PSquare;
  freed: Integer;

constructor TShape.Init(ax: Integer);
begin
  x := ax;
  next := nil;
end;

procedure TShape.Draw;
begin
  WriteLn('shape ', x);
end;

procedure TShape.Show;
begin
  Write('show: ');
  Draw;
end;

destructor TShape.Done;
begin
  freed := freed + 1;
end;

constructor TCircle.Init(ax, r: Integer);
begin
  TShape.Init(ax);
  radius := r;
end;

procedure TCircle.Draw;
begin
  WriteLn('circle ', x, ' radius ', radius);
end;

destructor TCircle.Done;
begin
  freed := freed + 10;
end;

procedure TSquare.Draw;
begin
  WriteLn('square ', x, ' side ', side);
end;

procedure Add(shape: PShape);
begin
  shape^.next := list;
  list := shape;
end;

begin
  freed := 0;
  list := nil;
  New(p, Init(1));
  Add(p);
  pc := New(PCircle, Init(2, 20));
  pc^.radius := pc^.radius + 1;
  Add(pc);
  New(ps, Init(3));
  ps^.side := 30;
  Add(ps);
  p := list;
  while p <> nil do
  begin
    p^.Draw;
    p^.Show;
    p := p^.next;
  end;
  while list <> nil do
  begin
    p := list;
    list := list^.next;
    Dispose(p, Done);
  end;
  WriteLn('freed ', freed);
end.
