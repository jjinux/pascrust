{ PRTurtle, Ruscal's turtle graphics on top of Graph, drawn without a
  window and checked with GetPixel. The expected output is worked out by
  hand (Free Pascal has no PRTurtle). The screen is 640 x 480, so the
  turtle's home (0, 0) is pixel (320, 240), and y goes up. }
program PRTurtleTest;
uses Graph, PRTurtle;
var
   driver, mode, i: Integer;
begin
   driver := Detect;
   InitGraph(driver, mode, '');
   WriteLn('home: ', Xcor, ' ', Ycor, ' ', Heading);
   { A square, north first, turning right. }
   for i := 1 to 4 do
   begin
      Forwd(100);
      TurnRight(90);
   end;
   WriteLn('square: ', Xcor, ' ', Ycor, ' ', Heading, ', ', GetPixel(320, 190), ' ', GetPixel(370, 140),
      ' ', GetPixel(420, 190), ' ', GetPixel(300, 100));
   { Pen up: moving draws nothing. }
   PenUp;
   SetHeading(West);
   Forwd(50);
   WriteLn('pen up: ', Xcor, ' ', GetPixel(295, 240));
   PenDown;
   SetPenColor(Red);
   Back(20);
   WriteLn('back: ', Xcor, ' ', GetPixel(280, 240));
   { TurnLeft from north is west; SetPosition moves without drawing. }
   SetHeading(North);
   TurnLeft(90);
   WriteLn('left: ', Heading);
   SetPosition(-200, -100);
   WriteLn('position: ', Xcor, ' ', Ycor, ' ', GetPixel(170, 290));
   { Wrap: off the right edge, back in on the left. }
   Wrap;
   SetPosition(300, 0);
   SetHeading(East);
   Forwd(40);
   WriteLn('wrap: ', Xcor, ' ', TurtleThere, ' ', GetPixel(630, 240), ' ', GetPixel(5, 240));
   NoWrap;
   Forwd(1000);
   WriteLn('nowrap: ', Xcor, ' ', TurtleThere);
   { Wrap from outside the window: the turtle comes back in first, and
     draws only from there (x = 60 to 70, pixels 380 to 390). }
   Wrap;
   SetPenColor(Yellow);
   Forwd(10);
   WriteLn('wrap from outside: ', Xcor, ' ', GetPixel(385, 240), ' ', GetPixel(100, 240) <> Yellow);
   SetPosition(1000, 0);
   Forwd(0);
   WriteLn('forwd(0) outside: ', Xcor);
   NoWrap;
   { Graph's own colour is left as it was. }
   SetColor(LightBlue);
   Forwd(5);
   WriteLn('colour: ', GetColor);
   { A turtle window: 100 x 100 around (100, 100); home is its centre. }
   TurtleWindow(100, 100, 100, 100);
   SetPenColor(Green);
   Forwd(30);
   WriteLn('window: ', Xcor, ' ', Ycor, ' ', GetPixel(0, 50), ' ', GetPixel(50, 20));
   { A window with no width or height is ignored (Wrap would loop
     forever in it). }
   TurtleWindow(100, 100, 0, 0);
   Wrap;
   Forwd(1);
   NoWrap;
   WriteLn('empty window: ', Xcor, ' ', Ycor);
   { The turtle itself, a triangle with its tip 10 pixels ahead (50, 40)
     and its back edge 5 behind (50, 55): shown, it's drawn (in XOR);
     hidden, it's gone. }
   ClearScreen;
   ShowTurtle;
   WriteLn('shown: ', GetPixel(50, 40), ' ', GetPixel(50, 55));
   HideTurtle;
   WriteLn('hidden: ', GetPixel(50, 40), ' ', GetPixel(50, 55));
   CloseGraph;
end.
