{t                              Shannon Behrens
                                6/8/97

This draws Ollie (a yellow) smily face given a center and a radius.
Must be in graphics mode.  For EraseOllie, please give the color of the
background.  All the real's are on a a 0..13 X scale and 0..10 Y scale}

Unit Ollie;

INTERFACE
   Uses Graph, Crt;
   Type
      OllieType = Object
         Procedure DrawOllie(XPos,YPos,Radius:Real);
         Procedure Frown(XPos,YPos,Radius:Real);
         Procedure Blink(XPos,YPos,Radius:Real);
         Procedure EraseOllie(XPos,YPos,Radius:real ; Color:word);
         end; {object}

IMPLEMENTATION
   Function XtoPix(x:real) : integer;
      {Create a horizontal scale of 0 to 13}
      Const MaxX = 13;
      begin
         XtoPix:= round((x/MaxX) * GetMaxX);
      end; {BoardDrawerType.XtoPix}

   Function YtoPix(y:real) : integer;
      {Create a vertical scale of 0 to 10}
      Const MaxY = 10;
      begin
         YtoPix:= round((y/MaxY) * GetMaxY);
      end; {BoardDrawertype.YtoPix}

   Procedure OllieType.DrawOllie(XPos,YPos,Radius:Real);
      var CurrentColor:word;
          OldFillPattern:FillPatternType;
      Procedure Initialize(var CurrentColor:word ; var
         OldFillPattern:FillPatternType);
         begin
            CurrentColor:= GetColor;
            setcolor(black);
            GetFillPattern(OldFillPattern);
            SetFillPattern(OldFillPattern,yellow);
         end; {Initialize}
      Procedure DeInitialize(CurrentColor:Word);
         {restore color and LineStyle}
         begin
            SetColor(CurrentColor);
            SetLineStyle(SolidLn,0,NormWidth);
         end; {DeInitialize}
      begin
         Initialize(CurrentColor,OldFillPattern);
         FillEllipse(XtoPix(XPos),YtoPix(YPos),    {draw face}
            XtoPix(Radius),YtoPix(Radius));
         SetFillPattern(OldFillPattern,black);     {Right eye and left eye}
         FillEllipse(XtoPix(Xpos+1/3.5*Radius),YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*radius),YtoPix(1/6*radius));
         FillEllipse(XtoPix(Xpos-1/3.5*Radius),YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*radius),YtoPix(1/6*radius));
         If radius>2.5 then                        {widen mouth}
            SetLineStyle(SolidLn,0,ThickWidth);
         Arc(XtoPix(XPos),YtoPix(YPos-1/3*Radius),235,305,XtoPix(Radius));
         DeInitialize(CurrentColor);
      end; {OllieType.DrawOllie}

   Procedure OllieType.Frown(XPos,YPos,Radius:Real);
      var CurrentColor:word;
          OldFillPattern:FillPatternType;
      Procedure Initialize(var CurrentColor:word ; var
         OldFillPattern:FillPatternType);
         begin
            CurrentColor:= GetColor;
            setcolor(black);
            GetFillPattern(OldFillPattern);
            SetFillPattern(OldFillPattern,yellow);
         end; {Initialize}
      Procedure DeInitialize(CurrentColor:Word);
         {restore color and LineStyle}
         begin
            SetColor(CurrentColor);
            SetLineStyle(SolidLn,0,NormWidth);
         end; {DeInitialize}
      begin
         Initialize(CurrentColor,OldFillPattern);
         FillEllipse(XtoPix(XPos),YtoPix(YPos),    {draw face}
            XtoPix(Radius),YtoPix(Radius));
         SetFillPattern(OldFillPattern,black);     {Right eye and left eye}
         FillEllipse(XtoPix(Xpos+1/3.5*Radius),YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*radius),YtoPix(1/6*radius));
         FillEllipse(XtoPix(Xpos-1/3.5*Radius),YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*radius),YtoPix(1/6*radius));
         If radius>2.5 then                        {widen mouth}
            SetLineStyle(SolidLn,0,ThickWidth);
         Arc(XtoPix(XPos),YtoPix(YPos+4/3*Radius),55,125,XtoPix(Radius));
         DeInitialize(CurrentColor);
      end; {OllieType.Frown}

   Procedure OllieType.Blink(XPos,YPos,Radius:Real);
      Const OllieBlinkTime = 1000;
            MaxBlinkHeight = 20;
            MinBlinkHeight = 7;
      var OldFillPattern:FillPatternType;
          a:integer;
      begin
         DrawOllie(XPos,YPos,Radius);              {Draw Him}
         GetFillPattern(OldFillPattern);
         SetFillPattern(OldFillPattern,yellow);    {Erase eye}
         FillEllipse(XtoPix(Xpos-1/3.5*Radius),YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*radius),YtoPix(1/6*radius));
         For a:= MaxBlinkHeight downto MinBlinkHeight do
            begin
            SetFillPattern(OldFillPattern,black);  {increasingly smaller}
            FillEllipse(XtoPix(Xpos-1/3.5*Radius),YtoPix(YPos-1/4*Radius),
               XtoPix(1/8*radius),YtoPix((1/6*radius)*(a/MaxBlinkHeight)));
            SetFillPattern(OldFillPattern,yellow);
            FillEllipse(XtoPix(Xpos-1/3.5*Radius),YtoPix(YPos-1/4*Radius),
               XtoPix(1/8*radius),YtoPix((1/6*radius)*(a/MaxBlinkHeight)));
            end; {for}
         SetFillPattern(OldFillPattern,black);  {increasingly smaller}
            FillEllipse(XtoPix(Xpos-1/3.5*Radius),YtoPix(YPos-1/4*Radius),
               XtoPix(1/8*radius),YtoPix((1/6*radius)*(a/MaxBlinkHeight)));
         Delay(OllieBlinkTime);
         SetFillPattern(OldFillPattern,black);
         For a:= MinBlinkHeight to MaxBlinkHeight do
            begin
            FillEllipse(XtoPix(Xpos-1/3.5*Radius),YtoPix(YPos-1/4*Radius),
               XtoPix(1/8*radius),YtoPix((1/6*radius)*(a/MaxBlinkHeight)));
            end; {for}
         Delay(OllieBlinkTime);
      end; {OllieType.Blink}

   Procedure OllieType.EraseOllie(XPos,YPos,Radius:real ; Color:word);
      var OldFillPattern:FillPatternType;
      begin
         GetFillPattern(OldFillPattern);
         SetFillPattern(OldFillPattern,black);
         FillEllipse(XtoPix(XPos),YtoPix(YPos),
            XtoPix(Radius),YtoPix(Radius));
      end; {OllieType.EraseOllie}

begin
   {no initialization}
end.
