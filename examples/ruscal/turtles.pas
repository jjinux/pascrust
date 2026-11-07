{ Turtles: three drawings with PRTurtle, Ruscal's turtle graphics unit,
  to watch being drawn. Press a key after each one.

  - A Koch snowflake: each side is four sides a third as long, again and
    again.
  - A tree: a trunk with two smaller trees on it, again and again.
  - A spiral of squares, each a little bigger and turned a little more.

  Written for Ruscal; it also compiles in Turbo Pascal 7 with
  PRTurtle.pas from Ruscal. }
program Turtles;

uses Crt, Graph, PRTurtle;

var
   Driver, Mode: Integer;

{ One side of the snowflake: at depth 0 a straight line, else four
  sides a third as long, with a bump in the middle. }
procedure KochSide(Length: Real; Depth: Integer);
   begin
      if Depth = 0 then
         Forwd(Round(Length))
      else
      begin
         KochSide(Length / 3, Depth - 1);
         TurnLeft(60);
         KochSide(Length / 3, Depth - 1);
         TurnRight(120);
         KochSide(Length / 3, Depth - 1);
         TurnLeft(60);
         KochSide(Length / 3, Depth - 1);
      end;
   end;

procedure Snowflake;
   var
      I: Integer;
   begin
      ClearScreen;
      SetPenColor(LightCyan);
      PenUp;
      SetPosition(-180, 100);
      SetHeading(East);
      PenDown;
      for I := 1 to 3 do
      begin
         KochSide(360, 4);
         TurnRight(120);
      end;
   end;

{ A tree: a trunk, then two smaller trees leaning left and right, and
  back down the trunk to where it started. }
procedure Tree(Length, Depth: Integer);
   begin
      if Depth = 0 then
         Exit;
      if Depth <= 2 then
         SetPenColor(LightGreen)
      else
         SetPenColor(Brown);
      Forwd(Length);
      TurnLeft(25);
      Tree(Length * 3 div 4, Depth - 1);
      TurnRight(50);
      Tree(Length * 3 div 4, Depth - 1);
      TurnLeft(25);
      PenUp;
      Back(Length);
      PenDown;
   end;

procedure Forest;
   begin
      ClearScreen;
      PenUp;
      SetPosition(0, -220);
      PenDown;
      Tree(110, 9);
   end;

procedure Spiral;
   var
      I, Side: Integer;
   begin
      ClearScreen;
      { The squares grow up and to the left of where they start. }
      PenUp;
      SetPosition(28, -60);
      PenDown;
      for I := 1 to 80 do
      begin
         SetPenColor(I mod 15 + 1);
         for Side := 1 to 4 do
         begin
            Forwd(I * 2);
            TurnRight(90);
         end;
         TurnRight(4);
      end;
   end;

procedure WaitForKey;
   begin
      HideTurtle;
      SetColor(White);
      OutTextXY(10, GetMaxY - 15, 'Press a key');
      if ReadKey = #0 then
         ReadKey;
   end;

begin
   Driver := Detect;
   InitGraph(Driver, Mode, '');
   TurtleDelay(1);
   ShowTurtle;
   Snowflake;
   WaitForKey;
   ShowTurtle;
   Forest;
   WaitForKey;
   ShowTurtle;
   Spiral;
   WaitForKey;
   CloseGraph;
end.
