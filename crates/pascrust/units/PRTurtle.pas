{ PRTurtle: turtle graphics on top of the Graph unit, with the routines
  of Turbo Pascal 3.0's turtle (Graph3), so a turtle program can use
  Graph and PRTurtle instead. Part of PascRust (MIT); written in Turbo
  Pascal, so it also compiles in TP 7 itself.

  The turtle starts at the centre of its window, heading north (up).
  Headings are in degrees, clockwise: North = 0, East = 90. Turtle
  coordinates have their origin at the window's centre, with y going up.
  Call InitGraph first; the window is the whole screen until
  TurtleWindow says otherwise. }
unit PRTurtle;

interface

   uses Graph;

   const
      North = 0;
      East = 90;
      South = 180;
      West = 270;

   procedure Back(Dist: Integer);
   procedure ClearScreen;
   procedure Forwd(Dist: Integer);
   function Heading: Integer;
   procedure HideTurtle;
   procedure Home;
   procedure NoWrap;
   procedure PenDown;
   procedure PenUp;
   procedure SetHeading(Angle: Integer);
   procedure SetPenColor(Color: Word);
   procedure SetPosition(X, Y: Integer);
   procedure ShowTurtle;
   procedure TurnLeft(Angle: Integer);
   procedure TurnRight(Angle: Integer);
   procedure TurtleDelay(Ms: Integer);
   function TurtleThere: Boolean;
   procedure TurtleWindow(X, Y, W, H: Integer);
   procedure Wrap;
   function Xcor: Integer;
   function Ycor: Integer;

implementation

   uses Crt;

   var
      Ready: Boolean;           { the window has been set up }
      PosX, PosY: Real;         { where the turtle is, in turtle coordinates }
      Dir: Real;                { its heading, in degrees from north }
      Down: Boolean;            { the pen draws }
      Shown: Boolean;           { the turtle is drawn }
      Wrapping: Boolean;        { leaving one side comes back on the other }
      Pen: Word;                { the pen's colour }
      Pause: Integer;           { milliseconds to wait after each move }
      WinLeft, WinTop, WinWidth, WinHeight: Integer;

   { The whole screen, the first time any routine is called (InitGraph has
     run by then, so GetMaxX knows the screen). }
   procedure Setup;
      begin
         if not Ready then
         begin
            Ready := True;
            WinLeft := 0;
            WinTop := 0;
            WinWidth := GetMaxX + 1;
            WinHeight := GetMaxY + 1;
            Pen := GetMaxColor;
            Down := True;
         end;
      end;

   { A turtle point on the screen, relative to the turtle window (which is
     the viewport). }
   function ScreenX(X: Real): Integer;
      begin
         ScreenX := WinWidth div 2 + Round(X);
      end;

   function ScreenY(Y: Real): Integer;
      begin
         ScreenY := WinHeight div 2 - Round(Y);
      end;

   { Flip a pixel's colour, as XOR mode does for lines. }
   procedure FlipPixel(X, Y: Integer);
      begin
         PutPixel(X, Y, GetPixel(X, Y) xor GetMaxColor);
      end;

   { The turtle is a small triangle pointing where it's heading, drawn in
     XOR mode, so drawing it a second time takes it away again. (Two lines
     meet at each corner, which XOR draws twice, clearing it; each corner is
     flipped once more.) }
   procedure DrawTurtle;
      var
         A, TipX, TipY, LeftX, LeftY, RightX, RightY: Real;
      begin
         A := Dir * Pi / 180;
         TipX := PosX + 10 * Sin(A);
         TipY := PosY + 10 * Cos(A);
         LeftX := PosX + 6 * Sin(A - 2.5);
         LeftY := PosY + 6 * Cos(A - 2.5);
         RightX := PosX + 6 * Sin(A + 2.5);
         RightY := PosY + 6 * Cos(A + 2.5);
         SetWriteMode(XORPut);
         SetColor(GetMaxColor);
         Line(ScreenX(TipX), ScreenY(TipY), ScreenX(LeftX), ScreenY(LeftY));
         Line(ScreenX(LeftX), ScreenY(LeftY), ScreenX(RightX), ScreenY(RightY));
         Line(ScreenX(RightX), ScreenY(RightY), ScreenX(TipX), ScreenY(TipY));
         SetWriteMode(NormalPut);
         FlipPixel(ScreenX(TipX), ScreenY(TipY));
         FlipPixel(ScreenX(LeftX), ScreenY(LeftY));
         FlipPixel(ScreenX(RightX), ScreenY(RightY));
      end;

   { Draw from the turtle to (ToX, ToY) if the pen is down, and move there. }
   procedure DrawTo(ToX, ToY: Real);
      begin
         if Down then
         begin
            SetColor(Pen);
            Line(ScreenX(PosX), ScreenY(PosY), ScreenX(ToX), ScreenY(ToY));
         end;
         PosX := ToX;
         PosY := ToY;
      end;

   { Move to (ToX, ToY). With Wrap on, a line that leaves the window goes
     on from the opposite side, as often as it has to. }
   procedure MoveTurtle(ToX, ToY: Real);
      var
         HalfW, HalfH, T, TX, TY, DX, DY: Real;
         Steps: Integer;
      begin
         HalfW := WinWidth / 2;
         HalfH := WinHeight / 2;
         Steps := 0;
         while Wrapping and ((Abs(ToX) > HalfW) or (Abs(ToY) > HalfH)) and (Steps < 100) do
         begin
            { Where the line first meets the window's edge: a fraction T of
              the way there. }
            DX := ToX - PosX;
            DY := ToY - PosY;
            TX := 2;
            TY := 2;
            if DX > 0 then
               TX := (HalfW - PosX) / DX
            else if DX < 0 then
               TX := (-HalfW - PosX) / DX;
            if DY > 0 then
               TY := (HalfH - PosY) / DY
            else if DY < 0 then
               TY := (-HalfH - PosY) / DY;
            if TX < TY then
               T := TX
            else
               T := TY;
            DrawTo(PosX + T * DX, PosY + T * DY);
            { Come back in on the other side. }
            if T = TX then
            begin
               if DX > 0 then
               begin
                  PosX := PosX - WinWidth;
                  ToX := ToX - WinWidth;
               end
               else
               begin
                  PosX := PosX + WinWidth;
                  ToX := ToX + WinWidth;
               end;
            end
            else if DY > 0 then
            begin
               PosY := PosY - WinHeight;
               ToY := ToY - WinHeight;
            end
            else
            begin
               PosY := PosY + WinHeight;
               ToY := ToY + WinHeight;
            end;
            Inc(Steps);
         end;
         DrawTo(ToX, ToY);
      end;

   procedure Forwd(Dist: Integer);
      var
         A: Real;
      begin
         Setup;
         if Shown then
            DrawTurtle;
         A := Dir * Pi / 180;
         MoveTurtle(PosX + Dist * Sin(A), PosY + Dist * Cos(A));
         if Shown then
            DrawTurtle;
         if Pause > 0 then
            Delay(Pause);
      end;

   procedure Back(Dist: Integer);
      begin
         Forwd(-Dist);
      end;

   { An angle kept between 0 and 360. }
   function Normal(A: Real): Real;
      begin
         while A < 0 do
            A := A + 360;
         while A >= 360 do
            A := A - 360;
         Normal := A;
      end;

   procedure Turn(Angle: Real);
      begin
         Setup;
         if Shown then
            DrawTurtle;
         Dir := Normal(Dir + Angle);
         if Shown then
            DrawTurtle;
      end;

   procedure TurnLeft(Angle: Integer);
      begin
         Turn(-Angle);
      end;

   procedure TurnRight(Angle: Integer);
      begin
         Turn(Angle);
      end;

   procedure SetHeading(Angle: Integer);
      begin
         Turn(Normal(Angle) - Dir);
      end;

   function Heading: Integer;
      begin
         Heading := Round(Dir) mod 360;
      end;

   procedure SetPosition(X, Y: Integer);
      begin
         Setup;
         if Shown then
            DrawTurtle;
         PosX := X;
         PosY := Y;
         if Shown then
            DrawTurtle;
      end;

   function Xcor: Integer;
      begin
         Xcor := Round(PosX);
      end;

   function Ycor: Integer;
      begin
         Ycor := Round(PosY);
      end;

   procedure Home;
      begin
         Setup;
         if Shown then
            DrawTurtle;
         PosX := 0;
         PosY := 0;
         Dir := North;
         if Shown then
            DrawTurtle;
      end;

   procedure ClearScreen;
      begin
         Setup;
         ClearViewPort;
         Shown := False;
         Home;
      end;

   procedure PenDown;
      begin
         Setup;
         Down := True;
      end;

   procedure PenUp;
      begin
         Setup;
         Down := False;
      end;

   procedure SetPenColor(Color: Word);
      begin
         Setup;
         Pen := Color;
      end;

   procedure ShowTurtle;
      begin
         Setup;
         if not Shown then
         begin
            Shown := True;
            DrawTurtle;
         end;
      end;

   procedure HideTurtle;
      begin
         Setup;
         if Shown then
         begin
            DrawTurtle;
            Shown := False;
         end;
      end;

   procedure Wrap;
      begin
         Wrapping := True;
      end;

   procedure NoWrap;
      begin
         Wrapping := False;
      end;

   procedure TurtleDelay(Ms: Integer);
      begin
         Pause := Ms;
      end;

   function TurtleThere: Boolean;
      begin
         Setup;
         TurtleThere := (Abs(PosX) <= WinWidth / 2) and (Abs(PosY) <= WinHeight / 2);
      end;

   { The turtle's window: W by H pixels with its centre at (X, Y) on the
     screen. It's the viewport, so lines are clipped to it, and the turtle
     goes home. }
   procedure TurtleWindow(X, Y, W, H: Integer);
      begin
         Setup;
         HideTurtle;
         WinLeft := X - W div 2;
         WinTop := Y - H div 2;
         WinWidth := W;
         WinHeight := H;
         SetViewPort(WinLeft, WinTop, WinLeft + W - 1, WinTop + H - 1, ClipOn);
         Home;
      end;

end.
