program UntypedObjects;
{ An object's field as an untyped argument starts at its offset in TP's
  layout, which counts the parent's fields and the hidden VMT field
  (Move(C.R, W, 2)). A method called on a typecast untyped parameter
  (TCounter(Data).Bump) changes the caller's variable. }
type
   TShape = object
      X, Y: Integer;
      constructor Init(AX, AY: Integer);
      procedure Draw; virtual;
   end;
   TCircle = object(TShape)
      R: Integer;
      procedure Draw; virtual;
   end;
   TCounter = object
      N: Integer;
      procedure Bump;
   end;

constructor TShape.Init(AX, AY: Integer);
   begin
      X := AX;
      Y := AY;
   end;

procedure TShape.Draw;
   begin
      WriteLn('shape');
   end;

procedure TCircle.Draw;
   begin
      WriteLn('circle ', X, ' ', Y, ' ', R);
   end;

procedure TCounter.Bump;
   begin
      Inc(N);
   end;

procedure Twice(var Data);
   begin
      TCounter(Data).Bump;
      TCounter(Data).Bump;
      WriteLn(TCounter(Data).N);
   end;

var
   C: TCircle;
   Counter: TCounter;
   W: Word;
begin
   C.Init(1, 2);
   C.R := 3;
   Move(C.R, W, 2);
   WriteLn(W);
   Move(C.Y, W, 2);
   WriteLn(W);
   Counter.N := 5;
   Twice(Counter);
   WriteLn(Counter.N);
end.
