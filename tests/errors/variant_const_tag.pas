{ TP: a typed constant of a variant record gives the fields of the
  variant its tag selects, in a nested variant part too. }
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
  TLine = record
    Id: Integer;
    case K: Byte of
      0: (X: Integer);
      1: (Len: Integer;
          case Dashed: Boolean of
            False: (Width: Byte);
            True: (Dash, Gap: Byte));
  end;
const
  Good: TRec = (Kind: 1; B: 'y');
  R: TRec = (Kind: 0; B: 'x');
  F: TFig = (S: Square; Radius: 3);
  Line: TLine = (Id: 1; K: 1; Len: 5; Dashed: True; Dash: 2; Gap: 1);
  Bad: TLine = (Id: 1; K: 1; Len: 5; Dashed: False; Dash: 2);
  Mixed: TLine = (Id: 1; K: 1; Len: 5; Dashed: True; Width: 2; Dash: 3);
  NoTag: TLine = (Id: 1; K: 1; Len: 5; Width: 2);
begin
  WriteLn(Good.B, R.B, F.Radius, Line.Dash, Bad.Dash, Mixed.Dash, NoTag.Width);
end.
