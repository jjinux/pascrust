program VariantErrors;
{ A variant's labels are constants of the tag's type, each selecting one
  variant; a typed constant gives one variant's fields and the tag. }
type
  Shape = (Circle, Rect);
  Fig = record
    case Kind: Shape of
      Circle: (R: Integer);
      Rect, Circle: (W: Integer);
      3: (Z: Integer);
  end;
  Pair = record
    case Kind: Boolean of
      True: (A: Integer);
      False: (B: Integer);
  end;
const
  Both: Pair = (Kind: True; A: 1; B: 2);
  NoTag: Pair = (A: 1);
begin
end.
