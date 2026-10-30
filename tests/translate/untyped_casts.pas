program UntypedCasts;
{ Writes through a typecast of an untyped parameter: an element of a Word
  array, a record's field, Inc of one, and a string[5] as a whole (which
  takes 6 bytes, not a full string's 256). Output from fpc -Mtp. }
type
   TWords = array[0..127] of Word;
   Rec = record a, b: Word; end;
   TName = string[5];
   NRec = record name: TName; n: Integer; end;
var
   r: Rec;
   nr: NRec;
procedure SetWords(var Data);
   begin
      TWords(Data)[0] := 1;
      TWords(Data)[1] := 2;
   end;
procedure SetField(var Data);
   begin
      Rec(Data).b := 77;
      Inc(Rec(Data).a, 10);
   end;
procedure SetName(var Data; const s: string);
   begin
      TName(Data) := s;
   end;
begin
   r.a := 0; r.b := 0;
   SetWords(r);
   WriteLn(r.a, ' ', r.b);
   SetField(r);
   WriteLn(r.a, ' ', r.b);
   nr.name := 'abc'; nr.n := 258;
   SetName(nr, 'xy');
   WriteLn(nr.name, ' ', nr.n);
end.
