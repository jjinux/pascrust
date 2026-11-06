{ Writes nested.dat, tagless.dat, mixed.dat, strs.dat and ints.dat, more
  fixtures of crates/pascrust/tests/json_tests.rs (from Fable's review):
  variant parts inside variants, a record without a tag whose variants
  share bytes (a string, an array, a Boolean and a nested tag), an object
  with a parent, sets that don't start at 0, every character, reals, and
  files of strings and of integers. }
program More;
type
   Shape = (Circle, Square, Tri);
   Nested = record
      Id: Byte;
      case Kind: Shape of
         Circle: (R: Integer;
                  case Sub: Boolean of
                     False: (P: Byte);
                     True: (Q: Word));
         Square: (S: Byte;
                  case Sub2: Byte of
                     0: (W1: Word);
                     1: (L1, H1: Byte));
         Tri: (T: LongInt);
   end;
   Tagless = record
      case Integer of
         0: (Str: string[3]);
         1: (Arr: array[0..3] of Byte);
         2: (Flag: Boolean;
             case Mode: Boolean of
                False: (M1: Byte);
                True: (M2: Char));
   end;
   Base = object
      X: Integer;
      procedure SetX(V: Integer);
   end;
   Derived = object(Base)
      Y: Byte;
   end;
   Point = record
      X, Y: Integer;
   end;
   Mixed = record
      Pts: array[1..2] of Point;
      Small: set of 10..20;
      Bytes: set of Byte;
      Name: string;
      Chars: array[0..255] of Char;
      Reals: array[1..6] of Real;
      D: Derived;
   end;
var
   FN: file of Nested;
   FT: file of Tagless;
   FM: file of Mixed;
   FS: file of string;
   FI: file of Integer;
   N: Nested;
   T: Tagless;
   M: Mixed;
   S: string;
   I: Integer;

procedure Base.SetX(V: Integer);
   begin
      X := V;
   end;

begin
   Assign(FN, 'nested.dat');
   Rewrite(FN);
   FillChar(N, SizeOf(N), 0);
   N.Id := 1;
   N.Kind := Circle;
   N.R := -2;
   N.Sub := False;
   N.P := 9;
   Write(FN, N);
   N.Id := 2;
   N.Sub := True;
   N.Q := 65535;
   Write(FN, N);
   FillChar(N, SizeOf(N), 0);
   N.Id := 3;
   N.Kind := Square;
   N.S := 4;
   N.Sub2 := 0;
   N.W1 := 258;
   Write(FN, N);
   N.Id := 4;
   N.Kind := Tri;
   N.T := -100000;
   Write(FN, N);
   Close(FN);

   Assign(FT, 'tagless.dat');
   Rewrite(FT);
   T.Arr[0] := 3;
   T.Arr[1] := 65;
   T.Arr[2] := 66;
   T.Arr[3] := 67;
   T.Str := 'ab';
   Write(FT, T);
   Close(FT);

   Assign(FM, 'mixed.dat');
   Rewrite(FM);
   FillChar(M, SizeOf(M), 0);
   M.Pts[1].X := 1;
   M.Pts[1].Y := 2;
   M.Pts[2].X := -3;
   M.Pts[2].Y := 4;
   M.Small := [10, 15, 20];
   M.Bytes := [0, 7, 8, 255];
   M.Name := 'A full string';
   for I := 0 to 255 do
      M.Chars[I] := Chr(I);
   M.Reals[1] := 0;
   M.Reals[2] := -1.5;
   M.Reals[3] := 1e-39;
   M.Reals[4] := 1.7e38;
   M.Reals[5] := 0.1;
   M.Reals[6] := 1234567890123.0;
   M.D.SetX(-7);
   M.D.Y := 200;
   Write(FM, M);
   Close(FM);

   Assign(FS, 'strs.dat');
   Rewrite(FS);
   S := 'hello';
   Write(FS, S);
   S := '';
   Write(FS, S);
   Close(FS);

   Assign(FI, 'ints.dat');
   Rewrite(FI);
   I := -1;
   Write(FI, I);
   I := 300;
   Write(FI, I);
   Close(FI);
   WriteLn('done ', SizeOf(Nested), ' ', SizeOf(Tagless), ' ', SizeOf(Mixed));
end.
