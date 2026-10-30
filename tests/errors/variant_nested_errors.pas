program VariantNestedErrors;
{ A variant part without a tag field inside one with a tag isn't
  supported yet: the outer part is flattened, so it has no bytes to keep
  the inner one's in. }
type
  Rec = record
    case Kind: Integer of
      0: (A: Integer);
      1: (B: Integer;
          case Integer of
            0: (L: LongInt);
            1: (W: Word));
  end;
begin
end.
