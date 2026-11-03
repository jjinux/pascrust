# PRTurtle: turtle graphics on top of Graph

`PRTurtle` is one of PascRust's own units (their names start with `PR`).
It gives a program turtle graphics, with the routines Turbo Pascal 3.0's
turtle had (in TP 4.0 to 7.0, the `Graph3` unit), drawn with the `Graph`
unit. So a Graph3 turtle program ports by changing its `uses` line and
starting graphics with `InitGraph`; a new program can use it too.

```pascal
program Square;
uses Graph, PRTurtle;
var
   Driver, Mode, I: Integer;
begin
   Driver := Detect;
   InitGraph(Driver, Mode, '');
   for I := 1 to 4 do
   begin
      Forwd(100);
      TurnRight(90);
   end;
   ReadLn;
   CloseGraph;
end.
```

For more, [`examples/pascrust/turtles.pas`](../examples/pascrust/turtles.pas)
draws a Koch snowflake, a tree and a spiral (`cargo run -p pascrust --
run examples/pascrust/turtles.pas`).

The unit is written in Turbo Pascal (`crates/pascrust/units/PRTurtle.pas`)
and comes with PascRust: `uses PRTurtle` finds it unless there's a file of
that name next to the program. It's translated with the program, like any
unit, so it also compiles in Turbo Pascal 7 itself.

## The turtle

The turtle starts at the centre of its window, heading north (up), with
its pen down, in the brightest colour, and hidden. Headings are degrees,
clockwise from north: the constants `North` (0), `East` (90), `South`
(180) and `West` (270). Turtle coordinates have their origin at the
window's centre and y going up; distances are pixels. The window is the
whole screen until `TurtleWindow` sets another.

| Routine | What it does |
|---|---|
| `Forwd(Dist)`, `Back(Dist)` | Move `Dist` pixels ahead or back, drawing if the pen is down. |
| `TurnLeft(Angle)`, `TurnRight(Angle)` | Turn `Angle` degrees. |
| `SetHeading(Angle)`, `Heading` | Set or read the heading (0 to 359). |
| `SetPosition(X, Y)`, `Xcor`, `Ycor` | Move without drawing, or read where the turtle is. |
| `Home` | Back to the centre, heading north. |
| `PenUp`, `PenDown` | Stop or start drawing as the turtle moves. |
| `SetPenColor(Color)` | The colour the pen draws with. |
| `ShowTurtle`, `HideTurtle` | Draw the turtle (a small triangle, in XOR mode) or take it away. |
| `Wrap`, `NoWrap` | With `Wrap`, a turtle leaving one side of the window comes back on the other (one already outside comes back in first); with `NoWrap` (the default), it goes on out of sight. |
| `TurtleThere` | True if the turtle is inside its window. |
| `TurtleWindow(X, Y, W, H)` | Use a `W` by `H` window centred on screen pixel (`X`, `Y`); it becomes the viewport (lines are clipped to it), and the turtle goes home. |
| `ClearScreen` | Clear the window and send the turtle home. |
| `TurtleDelay(Ms)` | Wait `Ms` milliseconds after each move, to watch it draw. |

## Differences from Graph3

TP 3's graphics were CGA's: 320×200 in 4 colours or 640×200 in 2, set up
with `GraphColorMode` or `HiRes`, and with `Plot`, `Draw` and friends
alongside the turtle. PRTurtle has only the turtle; the rest is `Graph`'s
(`InitGraph`, `PutPixel`, `Line`, ...), on its 640×480 screen. As in TP 3,
the turtle is hidden until `ShowTurtle`. The turtle leaves `Graph`'s
colour (`SetColor`) as it was, but its write mode at `NormalPut` (TP has
no way to ask what it was). Graph3's turtle is documented
rather than available to compare against, so details such as the
turtle's shape may differ.
