program TimerLoop;
{ The way to get a steady tick without a timer interrupt (see
  docs/timer_interrupts.md): a main loop that does a tick's work, then
  waits out the rest of the tick with Delay, measuring time with GetTime.
  The ticks counted don't depend on how fast the machine is. }
uses Crt, Dos;
const
   TickMs = 55; { the PC timer's 18.2 ticks a second }
var
   ticks: LongInt;
   lastTick, elapsed, current: LongInt;

{ Milliseconds since midnight. }
function NowMs: LongInt;
   var
      h, m, s, s100: Word;
   begin
      GetTime(h, m, s, s100);
      NowMs := ((LongInt(h) * 60 + m) * 60 + s) * 1000 + s100 * 10;
   end;

{ What the interrupt handler did. }
procedure Tick;
   begin
      Inc(ticks);
   end;

begin
   ticks := 0;
   lastTick := NowMs;
   repeat
      Tick;
      { ... the rest of the frame's work goes here ... }
      elapsed := NowMs - lastTick;
      if (elapsed >= 0) and (elapsed < TickMs) then
         Delay(TickMs - elapsed);
      { Delay may wait longer than asked, so look at the clock again. }
      current := NowMs;
      if (current < lastTick) or (current - lastTick >= 2 * TickMs) then
         lastTick := current { past midnight, or a tick behind: start again }
      else
         lastTick := lastTick + TickMs; { on time: keep a steady 18.2 a second }
   until ticks = 10;
   WriteLn(ticks, ' ticks');
end.
