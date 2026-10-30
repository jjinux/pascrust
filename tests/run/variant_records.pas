program VariantRecords;
{ Tagged variant records: fields of the variant in use, set and read,
  through `with` and as `var` arguments; a typed constant; a `Boolean` tag;
  and a `file of` round trip, which keeps only the variant the tag selects.
  Output from fpc -Mtp, except SizeOf(Fig): Free Pascal pads the field
  after the 1-byte tag, so it says 10 where TP says 9. }
type
   Shape = (Circle, Rect, Dot);
   Fig = record
      X, Y: Integer;
      case Kind: Shape of
         Circle: (R: Integer);
         Rect: (W, H: Integer);
         Dot: ();
   end;
   Account = record
      Name: string[10];
      case Open: Boolean of
         True: (Balance: LongInt);
         False: (Closed: Word);
   end;
const
   UnitCircle: Fig = (X: 0; Y: 0; Kind: Circle; R: 1);
   var
   f, g: Fig;
   a: Account;
   fl: file of Fig;

   procedure Grow(var n: Integer);
   begin
   n := n * 2;
   end;

   function Area(const Fig: Fig): Integer;
   begin
   case Fig.Kind of
    Circle: Area := 3 * Fig.R * Fig.R;
    Rect: Area := Fig.W * Fig.H;
   else
    Area := 0;
   end;
   end;

   begin
   f.X := 1;
   f.Y := 2;
   f.Kind := Rect;
   f.W := 3;
   f.H := 4;
   WriteLn(Area(f), ' ', SizeOf(Fig));
   with f do
    WriteLn(X + Y + W);
   Grow(f.H);
   WriteLn(Area(f));
   WriteLn(Area(UnitCircle), ' ', Ord(UnitCircle.Kind));
   a.Name := 'savings';
   a.Open := True;
   a.Balance := 100000;
   WriteLn(a.Name, ' ', a.Balance, ' ', SizeOf(Account));
   Assign(fl, 'variant_records.dat');
   Rewrite(fl);
   Write(fl, f);
   f.Kind := Circle;
   f.R := 7;
   Write(fl, f);
   f.Kind := Dot;
   Write(fl, f);
   Close(fl);
   Reset(fl);
   Read(fl, g);
   WriteLn(Ord(g.Kind), ' ', g.W, ' ', g.H);
   Read(fl, g);
   WriteLn(Ord(g.Kind), ' ', g.R, ' ', Area(g));
   Read(fl, g);
   WriteLn(Ord(g.Kind), ' ', Area(g), ' ', FileSize(fl));
   Close(fl);
   Erase(fl);
   end.
