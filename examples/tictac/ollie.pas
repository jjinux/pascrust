{t                              Shannon Behrens
                                6/8/97

This draws Ollie (a yellow) smily face given a center and a radius.
Must be in graphics mode.  For EraseOllie, please give the color of the
background.  All the real's are on a a 0..13 X scale and 0..10 Y scale}

unit Ollie;

interface
   uses Graph, Crt;
   type
      OllieType = object
         procedure DrawOllie(XPos, YPos, Radius: real);
         procedure Frown(XPos, YPos, Radius: real);
         procedure blink(XPos, YPos, Radius: real);
         procedure EraseOllie(XPos, YPos, Radius: real; Color: word);
      end; {object}

implementation
   function XtoPix(X: real): integer;
      {Create a horizontal scale of 0 to 13}
      const MaxX = 13;
      begin
         XtoPix := round((X/MaxX) * GetMaxX);
      end; {BoardDrawerType.XtoPix}

   function YtoPix(Y: real): integer;
      {Create a vertical scale of 0 to 10}
      const MaxY = 10;
      begin
         YtoPix := round((Y/MaxY) * GetMaxY);
      end; {BoardDrawertype.YtoPix}

   procedure OllieType.DrawOllie(XPos, YPos, Radius: real);
      var CurrentColor: word;
         OldFillPattern: FillPatternType;
      procedure Initialize(var CurrentColor: word; var
         OldFillPattern: FillPatternType);
         begin
            CurrentColor := GetColor;
            SetColor(black);
            GetFillPattern(OldFillPattern);
            SetFillPattern(OldFillPattern, yellow);
         end; {Initialize}
      procedure DeInitialize(CurrentColor: word);
         {restore color and LineStyle}
         begin
            SetColor(CurrentColor);
            SetLineStyle(SolidLn, 0, NormWidth);
         end; {DeInitialize}
      begin
         Initialize(CurrentColor, OldFillPattern);
         FillEllipse(XtoPix(XPos), YtoPix(YPos),   {draw face}
            XtoPix(Radius), YtoPix(Radius));
         SetFillPattern(OldFillPattern, black);    {Right eye and left eye}
         FillEllipse(XtoPix(XPos+1/3.5*Radius), YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*Radius), YtoPix(1/6*Radius));
         FillEllipse(XtoPix(XPos-1/3.5*Radius), YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*Radius), YtoPix(1/6*Radius));
         if Radius>2.5 then                        {widen mouth}
            SetLineStyle(SolidLn, 0, ThickWidth);
         Arc(XtoPix(XPos), YtoPix(YPos-1/3*Radius), 235, 305, XtoPix(Radius));
         DeInitialize(CurrentColor);
      end; {OllieType.DrawOllie}

   procedure OllieType.Frown(XPos, YPos, Radius: real);
      var CurrentColor: word;
         OldFillPattern: FillPatternType;
      procedure Initialize(var CurrentColor: word; var
         OldFillPattern: FillPatternType);
         begin
            CurrentColor := GetColor;
            SetColor(black);
            GetFillPattern(OldFillPattern);
            SetFillPattern(OldFillPattern, yellow);
         end; {Initialize}
      procedure DeInitialize(CurrentColor: word);
         {restore color and LineStyle}
         begin
            SetColor(CurrentColor);
            SetLineStyle(SolidLn, 0, NormWidth);
         end; {DeInitialize}
      begin
         Initialize(CurrentColor, OldFillPattern);
         FillEllipse(XtoPix(XPos), YtoPix(YPos),   {draw face}
            XtoPix(Radius), YtoPix(Radius));
         SetFillPattern(OldFillPattern, black);    {Right eye and left eye}
         FillEllipse(XtoPix(XPos+1/3.5*Radius), YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*Radius), YtoPix(1/6*Radius));
         FillEllipse(XtoPix(XPos-1/3.5*Radius), YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*Radius), YtoPix(1/6*Radius));
         if Radius>2.5 then                        {widen mouth}
            SetLineStyle(SolidLn, 0, ThickWidth);
         Arc(XtoPix(XPos), YtoPix(YPos+4/3*Radius), 55, 125, XtoPix(Radius));
         DeInitialize(CurrentColor);
      end; {OllieType.Frown}

   procedure OllieType.blink(XPos, YPos, Radius: real);
      const OllieBlinkTime = 1000;
         MaxBlinkHeight = 20;
         MinBlinkHeight = 7;
      var OldFillPattern: FillPatternType;
         a: integer;
      begin
         DrawOllie(XPos, YPos, Radius);            {Draw Him}
         GetFillPattern(OldFillPattern);
         SetFillPattern(OldFillPattern, yellow);   {Erase eye}
         FillEllipse(XtoPix(XPos-1/3.5*Radius), YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*Radius), YtoPix(1/6*Radius));
         for a := MaxBlinkHeight downto MinBlinkHeight do
         begin
            SetFillPattern(OldFillPattern, black); {increasingly smaller}
            FillEllipse(XtoPix(XPos-1/3.5*Radius), YtoPix(YPos-1/4*Radius),
               XtoPix(1/8*Radius), YtoPix((1/6*Radius)*(a/MaxBlinkHeight)));
            SetFillPattern(OldFillPattern, yellow);
            FillEllipse(XtoPix(XPos-1/3.5*Radius), YtoPix(YPos-1/4*Radius),
               XtoPix(1/8*Radius), YtoPix((1/6*Radius)*(a/MaxBlinkHeight)));
         end; {for}
         SetFillPattern(OldFillPattern, black); {increasingly smaller}
         FillEllipse(XtoPix(XPos-1/3.5*Radius), YtoPix(YPos-1/4*Radius),
            XtoPix(1/8*Radius), YtoPix((1/6*Radius)*(a/MaxBlinkHeight)));
         delay(OllieBlinkTime);
         SetFillPattern(OldFillPattern, black);
         for a := MinBlinkHeight to MaxBlinkHeight do
         begin
            FillEllipse(XtoPix(XPos-1/3.5*Radius), YtoPix(YPos-1/4*Radius),
               XtoPix(1/8*Radius), YtoPix((1/6*Radius)*(a/MaxBlinkHeight)));
         end; {for}
         delay(OllieBlinkTime);
      end; {OllieType.Blink}

   procedure OllieType.EraseOllie(XPos, YPos, Radius: real; Color: word);
      var OldFillPattern: FillPatternType;
      begin
         GetFillPattern(OldFillPattern);
         SetFillPattern(OldFillPattern, black);
         FillEllipse(XtoPix(XPos), YtoPix(YPos),
            XtoPix(Radius), YtoPix(Radius));
      end; {OllieType.EraseOllie}

begin
   {no initialization}
end.
