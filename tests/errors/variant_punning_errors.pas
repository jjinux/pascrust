program VariantPunningErrors;
{ Two fields of one variant part kept as bytes can't be var arguments of
  one call (Rust borrows the part as a whole); a tagless variant part with
  a pointer can't be bytes, so it can't go in a file; a set in a tagless
  part isn't supported yet. }
type
  Pun = record
    case Integer of
      0: (L: LongInt);
      1: (Lo, Hi: Word);
  end;
  PNode = ^Node;
  Node = record
    case Integer of
      0: (Next: PNode);
      1: (Value: LongInt);
  end;
  Flags = record
    case Integer of
      0: (S: set of 0..7);
      1: (B: Byte);
  end;
var
  p: Pun;
  f: file of Node;

procedure Swap(var a, b: Word);
var
  t: Word;
begin
  t := a;
  a := b;
  b := t;
end;

begin
  Swap(p.Lo, p.Hi);
end.
