program InterruptVectors;
{ GetIntVec and SetIntVec, faked: a vector holds what a program puts in it
  (here @Tick, an interrupt procedure), and gives back DOS's handler, so
  saving and restoring a vector works, but PascRust has no interrupts and
  never calls a handler. Written by hand: Free Pascal on this machine has
  no interrupt vectors. }
uses Dos;
var
   old, cur, other: Pointer;
   ticks: Integer;

procedure Tick; interrupt;
   begin
      Inc(ticks);
   end;

procedure KeyHandler(Flags, CS, IP, AX, BX, CX, DX, SI, DI, DS, ES, BP: Word); interrupt;
   begin
      WriteLn('never');
   end;

begin
   ticks := 0;
   GetIntVec($1C, old);
   GetIntVec($09, other);
   WriteLn(old <> nil, ' ', old <> other);
   SetIntVec($1C, @Tick);
   SetIntVec($09, @KeyHandler);
   GetIntVec($1C, cur);
   WriteLn(cur = @Tick, ' ', cur <> old, ' ', cur <> @KeyHandler);
   SetIntVec($1C, old);
   GetIntVec($1C, cur);
   WriteLn(cur = old, ' ', ticks);
end.
