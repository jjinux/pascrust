# Timer interrupts

Many DOS games got a steady clock from the PC's timer: they installed their
own handler for interrupt `$1C` (or `$08`), which DOS called 18.2 times a
second, and counted ticks in it. PascRust has no interrupts, so
`SetIntVec($1C, @Tick)` keeps the handler but never calls it, and the
translation warns about it.

To get the same steady tick, call the handler yourself from the program's
main loop, and wait out the rest of each tick with `Delay`, keeping track
of when the last tick ended. `Delay` may wait a little longer than asked,
so the loop looks at the clock again afterwards: on time, the next tick
ends one tick after this one ended (so the ticks don't drift); a whole
tick or more behind (a slow frame), or past midnight (when the clock goes
back to 0), it starts again from the clock instead of rushing through the
missed ticks. This is plain Turbo Pascal, so it runs the same way in TP:

```pascal
uses Crt, Dos;
const
  TickMs = 55; { the PC timer's 18.2 ticks a second }
var
  lastTick, elapsed, current: LongInt;

{ Milliseconds since midnight. }
function NowMs: LongInt;
var
  h, m, s, s100: Word;
begin
  GetTime(h, m, s, s100);
  NowMs := ((LongInt(h) * 60 + m) * 60 + s) * 1000 + s100 * 10;
end;

begin
  lastTick := NowMs;
  repeat
    Tick; { what the interrupt handler did }
    { ... the rest of the frame's work ... }
    elapsed := NowMs - lastTick;
    if (elapsed >= 0) and (elapsed < TickMs) then
      Delay(TickMs - elapsed);
    { Delay may wait longer than asked, so look at the clock again. }
    current := NowMs;
    if (current < lastTick) or (current - lastTick >= 2 * TickMs) then
      lastTick := current { past midnight, or a tick behind: start again }
    else
      lastTick := lastTick + TickMs; { on time: keep a steady 18.2 a second }
  until Done;
end.
```

Then take out the `GetIntVec`/`SetIntVec` calls and the handler's
`interrupt` directive.
[`tests/run/timer_loop.pas`](../tests/run/timer_loop.pas) is this loop as
a whole program.
