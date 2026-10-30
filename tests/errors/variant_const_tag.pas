{ TP: a typed constant of a variant record gives the fields of the
  variant its tag selects. }
program VariantConstTag;
type
  Shape = (Circle, Square);
  TRec = record
    case Kind: Byte of
      0: (A: Integer);
      1: (B: Char);
  end;
  TFig = record
    case S: Shape of
      Circle: (Radius: Integer);
      Square: (Side: Integer);
  end;
const
  Good: TRec = (Kind: 1; B: 'y');
  R: TRec = (Kind: 0; B: 'x');
  F: TFig = (S: Square; Radius: 3);
begin
  WriteLn(Good.B, R.B, F.Radius);
end.
