{ Rust calling back into Pascal: the program passes its own function to
  a Rust routine as a procedural value, and the Rust sorts with it.
  Output written by hand: Free Pascal can't link Rust. }
program RustCallback;
{$L rust_callback.rs}
type
   TLess = function(A, B: Integer): Boolean;
   TList = array[1..6] of Integer;

procedure SortWith(var Items: TList; Less: TLess); external;

function Descending(A, B: Integer): Boolean;
   begin
      Descending := A > B;
   end;

var
   Items: TList;
   I: Integer;
begin
   Items[1] := 3; Items[2] := 1; Items[3] := 4; Items[4] := 1; Items[5] := 5; Items[6] := 9;
   SortWith(Items, Descending);
   for I := 1 to 6 do
      Write(Items[I], ' ');
   WriteLn;
end.
