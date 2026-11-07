program ObjectVmt;
{ Turbo Pascal gives an object with a constructor, a destructor or a
  virtual method a hidden 2-byte field, the address of its table of virtual
  methods (VMT). It follows the fields of the first type in the line to need
  one, before a descendant's own fields. SizeOf counts it, and so does a
  typed file. (Expected output by hand: 64-bit Free Pascal's VMT field is an
  8-byte pointer. TP writes the VMT's address; PascRust writes zeros.) }
type
   TPlain = object
      X, Y: Integer;
      procedure Show;
   end;
   TCtor = object
      X, Y: Integer;
      constructor Init;
   end;
   TDtor = object
      X: Integer;
      destructor Done;
   end;
   TShape = object
      X, Y: Integer;
      constructor Init(AX, AY: Integer);
      procedure Draw; virtual;
   end;
   TCircle = object(TShape)
      R: Integer;
      procedure Draw; virtual;
   end;
   TPlainChild = object(TPlain)
      Z: Integer;
      constructor Init;
   end;

procedure TPlain.Show;
   begin
   end;

constructor TCtor.Init;
   begin
   end;

destructor TDtor.Done;
   begin
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

constructor TPlainChild.Init;
   begin
   end;

var
   F: file of TCircle;
   B: file of Byte;
   C: TCircle;
   Value: Byte;
begin
   WriteLn(SizeOf(TPlain), ' ', SizeOf(TCtor), ' ', SizeOf(TDtor), ' ', SizeOf(TShape), ' ', SizeOf(TCircle),
      ' ', SizeOf(TPlainChild));
   C.Init(1, 2);
   C.R := 3;
   Assign(F, 'object_vmt.dat');
   Rewrite(F);
   Write(F, C);
   C.Init(4, 5);
   C.R := 6;
   Write(F, C);
   WriteLn(FileSize(F));
   Close(F);
   { The bytes: X, Y, the VMT field, then R. }
   Assign(B, 'object_vmt.dat');
   Reset(B);
   while not Eof(B) do
   begin
      Read(B, Value);
      Write(Value, ' ');
   end;
   WriteLn;
   Close(B);
   Reset(F);
   Seek(F, 1);
   Read(F, C);
   C.Draw;
   Close(F);
   Erase(F);
end.
