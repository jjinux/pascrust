program VirtualOverrides;
{ An override must be virtual too, and hiding a static method that calls a
  virtual one isn't supported. }
type
  TShape = object
    procedure Draw; virtual;
    procedure Show;
  end;
  TCircle = object(TShape)
    procedure Draw;
    procedure Show;
  end;

procedure TShape.Draw; begin end;
procedure TShape.Show; begin Draw end;
procedure TCircle.Draw; begin end;
procedure TCircle.Show; begin end;

begin
end.
