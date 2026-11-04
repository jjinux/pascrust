{ Loops only Exit leaves (while True, repeat ... until False): what follows
  them never runs, and a function after one has no result to give; and
  a repeat body ending in dead code after a Break. No unreachable code
  in the Rust. }
program EndlessLoops;
var
   k: Integer;

function ReadNumber: Integer;
   var
      N: Integer;
   begin
      N := 0;
      while True do
      begin
         Inc(N);
         if N = 3 then
         begin
            ReadNumber := N;
            Exit;
         end;
         WriteLn('try again');
      end;
   end;

procedure Spin;
   begin
      repeat
         Inc(k);
         if k > 2 then
            Exit;
      until False;
      WriteLn('never');
   end;

begin
   k := 0;
   WriteLn(ReadNumber);
   Spin;
   repeat
      Inc(k);
      Break;
      k := 9;
   until False;
   while True do
   begin
      Inc(k);
      if k > 6 then
         Break;
   end;
   WriteLn(k);
end.
