program IndexCalls;
{ An index that calls a function: in an assignment's target it's worked
  out first (Pick reads the array the target indexes), on both sides it
  runs twice, as in TP and Free Pascal, and in a method an index that
  calls a method reads fine. }
type
  Bag = object
    items: array[1..3] of Char;
    n: Integer;
    procedure Init;
    function Count: Integer;
    function Last: Char;
  end;
var
  arr: array[1..3] of Integer;
  counts: array[1..3] of Integer;
  calls: Integer;
  b: Bag;

function Pick: Integer;
begin
  Pick := arr[1];
end;

function Next: Integer;
begin
  calls := calls + 1;
  Next := calls;
end;

procedure Bag.Init;
begin
  items[1] := 'x';
  items[2] := 'y';
  items[3] := 'z';
  n := 2;
end;

function Bag.Count: Integer;
begin
  Count := n;
end;

function Bag.Last: Char;
begin
  Last := items[Count];
end;

begin
  arr[1] := 2;
  arr[2] := 0;
  arr[3] := 0;
  arr[Pick] := 7;
  WriteLn(arr[1], ' ', arr[2], ' ', arr[3]);
  counts[1] := 0;
  counts[2] := 0;
  counts[3] := 0;
  calls := 0;
  counts[Next] := counts[Next] + 5;
  WriteLn(counts[1], ' ', counts[2], ' ', counts[3], ' ', calls);
  b.Init;
  WriteLn(b.Last);
end.
