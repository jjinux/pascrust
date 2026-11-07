{ High and Low of an array are its bounds in the Rust (a.high(),
  TArr::HIGH), so they follow the array's declaration. }
program ArrayHighLow;
const
   N = 10;
type
   TArr = array[1..N] of Integer;
var
   a: TArr;
   i, total: Integer;
   l: LongInt;

procedure Last;
   begin
      WriteLn(a[High(a)]);
   end;

begin
   for i := Low(a) to High(a) do
      a[i] := i;
   total := 0;
   for i := 1 to High(a) do
      total := total + a[i];
   l := High(a) - Low(a) + 1;
   WriteLn(total, ' ', l, ' ', High(TArr), ' ', Low(a));
   if i < High(a) then
      WriteLn('less');
   Last;
end.
