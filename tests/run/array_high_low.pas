{ High and Low of an array are its bounds in the Rust (a.high(),
  TArr::HIGH), so they follow the array's declaration. }
program ArrayHighLow;
const
   N = 10;
type
   TArr = array[1..N] of Integer;
   TRec = record
      Items: array[0..N - 1] of Integer;
   end;
var
   a: TArr;
   r: TRec;
   i, total: Integer;
   l: LongInt;

procedure Last;
   begin
      WriteLn(a[High(a)]);
   end;

{ A loop variable used only in the loop, over a field's bounds. }
procedure Fields;
   var
      j, k: Integer;
   begin
      for k := Low(a) to High(a) do
         Write(a[k]);
      WriteLn;
      for j := Low(r.Items) to High(r.Items) do
         r.Items[j] := j * 10;
      with r do
         for j := High(Items) downto Low(Items) do
            Write(Items[j], ' ');
      WriteLn;
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
   Fields;
end.
