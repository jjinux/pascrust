# Timer interrupts

Many DOS games got a steady clock from the PC's timer: they installed their
own handler for interrupt `$1C` (or `$08`), which DOS called 18.2 times a
second, and counted ticks in it. PascRust has no interrupts, so
`SetIntVec($1C, @Tick)` keeps the handler but never calls it, and the
translation warns about it.

To get the same steady tick, call the handler yourself from the program's
main loop, and wait out the rest of each tick with `Delay`, keeping track
of when the last tick ended. This is plain Turbo Pascal, so it runs the
same way in TP:

```pascal
uses Crt, Dos;
const
  TickMs = 55; { the PC timer's 18.2 ticks a second }
var
  lastTick, elapsed: LongInt;

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
    lastTick := lastTick + TickMs;
  until Done;
end.
```

Then take out the `GetIntVec`/`SetIntVec` calls and the handler's
`interrupt` directive.
[`tests/run/timer_loop.pas`](../tests/run/timer_loop.pas) is this loop as
a whole program.
