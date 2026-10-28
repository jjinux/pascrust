{t                              Shannon Behrens
                                6/8/97

This is a menu window in graphics mode.  It appears basically like a windows
window (but simpler) and is white on a black background.  The scaling used is
0..5 X and 0..9 Y.  Restrictions:  must be in graphics mode}

unit Menu;

interface
   uses MainUnit, Graph, Crt;
   const MaxNumberOfOptions = 9;
      LineWidth = 0.25;                 {for top StatusLine}
   type
      OptionsType = array[1..MaxNumberOfOptions] of string[45];
      MenuType = object(StatusLineType)
         Options    : OptionsType;         {protected}
         Title      : string[40];
         TopMargin  : real;
         SideMargin : real;
         procedure Create(TM, SM: real);   {public}
         procedure ChangeTitle(NewTitle: string);
         procedure AddOption(NewOption: string);
         function GetChoice: char;
         procedure DisplayTopLine(TheLine: string);
         procedure WriteAsText;
      end; {object}

implementation

   function XtoPix(X: real): integer;
      {Create a horizontal scale of 0 to 5}
      const MaxX = 5;
      begin
         XtoPix := round((X/MaxX) * GetMaxX);
      end; {BoardDrawerType.XtoPix}

   function YtoPix(Y: real): integer;
      {Create a vertical scale of 0 to 9}
      const MaxY = 9;
      begin
         YtoPix := round((Y/MaxY) * GetMaxY);
      end; {BoardDrawertype.YtoPix}

   procedure MenuType.Create(TM, SM: real);
      var a: integer;
         OldFillPattern: FillPatternType;
      begin
         TopMargin := TM; SideMargin := SM;
         setbkcolor(white);
         ClearViewPort;
         Title := '';
         for a := 1 to MaxNumberOfOptions do
            Options[a] := '';
         GetFillPattern(OldFillPattern);
         SetFillPattern(OldFillPattern, blue);                      {Top Bar}
         bar(XtoPix(0+SideMargin*0.99), YtoPix(0+TopMargin),
            XtoPix(5-SideMargin*0.99), YtoPix(0+TopMargin+LineWidth));
         SetFillPattern(OldFillPattern, white);
         SetColor(blue);
         bar(XtoPix(0+SideMargin), YtoPix(0+TopMargin+LineWidth),   {white area}
            XtoPix(5-SideMargin), YtoPix(9-3/2*TopMargin));
         SetLineStyle(SolidLn, 0, ThickWidth);
         rectangle(XtoPix(0+SideMargin), YtoPix(0+TopMargin+LineWidth),
            XtoPix(5-SideMargin), YtoPix(9-3/2*TopMargin));
         SetLineStyle(SolidLn, 0, NormWidth);
      end; {MenuType.Create}

   procedure MenuType.ChangeTitle(NewTitle: string);
      begin
         Title := NewTitle;
      end; {MenuType.ChangeTitle}

   procedure MenuType.AddOption(NewOption: string);
      var a: integer;
      begin
         a := 1;
         while (a <= MaxNumberOfOptions) and (Options[a] <> '') do
            inc(a);
         Options[a] := NewOption;
      end; {MenuType.AddOptions}

   procedure MenuType.DisplayTopLine(TheLine: string);
      var OldFillPattern: FillPatternType;
      begin
         GetFillPattern(OldFillPattern);
         SetFillPattern(OldFillPattern, blue);
         bar(XtoPix(0+SideMargin*0.99), YtoPix(0+TopMargin), {erase old line}
            XtoPix(5-SideMargin*0.99), YtoPix(0+TopMargin+LineWidth));
         SetColor(yellow);
         SetTextJustify(centerText, centerText);
         OutTextXY(XtoPix(2.5), YtoPix(TopMargin+1/2*LineWidth), TheLine);
      end; {Menu.DisplayTopLine}

   function MenuType.GetChoice: char;
      var a: integer; b: char;             {a and b were the only way I could}
      begin                                {make the +(concat) work}
         DisplayTopLine(Title);
         SetColor(blue);
         a := 1; b := '1';
         while (Options[a] <> '') and (a <= MaxNumberOfOptions) do
         begin
            SetTextJustify(LeftText, centerText);
            OutTextXY(XtoPix(SideMargin+0.67), YtoPix(TopMargin+3*LineWidth+1/4*a),
               (b + '.  ' + Options[a]));
            inc(a); inc(b);
         end; {while}
         OutTextXY(XtoPix(SideMargin+1.5), YtoPix(TopMargin+3*LineWidth+1/3*a),
            ('Please Choose:  '));
         GetChoice := upcase(readkey);
      end; {MenuType.GetChoice}

   procedure MenuType.WriteAsText;
      var a: integer;
      begin
         DisplayTopLine(Title);
         SetColor(blue);
         a := 1;
         while (Options[a] <> '') and (a <= MaxNumberOfOptions) do
         begin
            SetTextJustify(LeftText, centerText);
            OutTextXY(XtoPix(SideMargin+0.2), YtoPix(TopMargin+3*LineWidth+1/4*a),
               Options[a]);
            inc(a);
         end; {while}
      end; {WriteAsText}

begin
   {none}
end.
