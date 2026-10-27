{t                              Shannon Behrens
                                6/8/97

This is a menu window in graphics mode.  It appears basically like a windows
window (but simpler) and is white on a black background.  The scaling used is
0..5 X and 0..9 Y.  Restrictions:  must be in graphics mode}

Unit Menu;

INTERFACE
Uses MainUnit, Graph, Crt;
Const MaxNumberOfOptions = 9;
      LineWidth=0.25;                   {for top StatusLine}
Type
   OptionsType = array[1..MaxNumberOfOptions] of string[45];
   MenuType = object(StatusLineType)
      options : optionsType;            {protected}
      title   : string[40];
      TopMargin : real;
      SideMargin : real;
      Procedure Create(TM,SM:Real);     {public}
      Procedure ChangeTitle(NewTitle:string);
      Procedure AddOption(NewOption:string);
      Function GetChoice:char;
      Procedure DisplayTopLine(TheLine:string);
      Procedure WriteAsText;
      end; {object}

IMPLEMENTATION

Function XtoPix(x:real) : integer;
   {Create a horizontal scale of 0 to 5}
   Const MaxX = 5;
   begin
      XtoPix:= round((x/MaxX) * GetMaxX);
   end; {BoardDrawerType.XtoPix}

Function YtoPix(y:real) : integer;
   {Create a vertical scale of 0 to 9}
   Const MaxY = 9;
   begin
      YtoPix:= round((y/MaxY) * GetMaxY);
   end; {BoardDrawertype.YtoPix}

Procedure MenuType.Create(TM,SM:Real);
   var a:integer;
       OldFillPattern:FillPatternType;
   begin
      TopMargin:= TM; SideMargin:= SM;
      setbkcolor(white);
      ClearViewPort;
      Title:= '';
      for a:= 1 to MaxNumberOfOptions do
         Options[a]:= '';
      GetFillPattern(OldFillPattern);
      SetFillPattern(OldFillPattern,blue);                       {Top Bar}
      bar(XtoPix(0+Sidemargin*0.99),YtoPix(0+TopMargin),
         XtoPix(5-Sidemargin*0.99),YtoPix(0+TopMargin+LineWidth));
      SetFillPattern(OldFillPattern,white);
      SetColor(blue);
      Bar(XtoPix(0+Sidemargin),YtoPix(0+TopMargin+LineWidth),    {white area}
         XtoPix(5-SideMargin),YtoPix(9-3/2*TopMargin));
      SetLineStyle(SolidLn,0,ThickWidth);
      rectangle(XtoPix(0+Sidemargin),YtoPix(0+TopMargin+LineWidth),
         XtoPix(5-SideMargin),YtoPix(9-3/2*TopMargin));
      SetLineStyle(SolidLn,0,NormWidth);
    end; {MenuType.Create}

Procedure MenuType.ChangeTitle(NewTitle:string);
   begin
      Title:= NewTitle;
   end; {MenuType.ChangeTitle}

Procedure MenuType.AddOption(NewOption:string);
   var a:integer;
   begin
      a:= 1;
      while (a<=MaxNumberOfOptions) and (Options[a]<>'') do
         inc(a);
      Options[a]:= NewOption;
   end; {MenuType.AddOptions}

Procedure MenuType.DisplayTopLine(TheLine:string);
   var OldFillPattern:FillPatternType;
   begin
      GetFillPattern(OldFillPattern);
      SetFillPattern(OldFillPattern,blue);
      bar(XtoPix(0+Sidemargin*0.99),YtoPix(0+TopMargin),  {erase old line}
         XtoPix(5-Sidemargin*0.99),YtoPix(0+TopMargin+LineWidth));
      SetColor(yellow);
      SetTextJustify(centerText,centerText);
      OutTextXY(XToPix(2.5),YToPix(TopMargin+1/2*LineWidth),TheLine);
   end; {Menu.DisplayTopLine}

Function MenuType.GetChoice:char;
   var a:integer; b:char;               {a and b were the only way I could}
   begin                                {make the +(concat) work}
      DisplayTopLine(title);
      setcolor(blue);
      a:= 1; b:= '1';
      while (options[a] <> '') and (a<=MaxNumberOfOptions) do
         begin
         SetTextJustify(LeftText,centerText);
         OutTextXY(XtoPix(SideMargin+0.67),YtoPix(TopMargin+3*LineWidth+1/4*a),
            (b + '.  ' + options[a]));
         inc(a); inc(b);
         end; {while}
      OutTextXY(XtoPix(SideMargin+1.5),YtoPix(TopMargin+3*LineWidth+1/3*a),
         ('Please Choose:  '));
      GetChoice:= upcase(readkey);
   end; {MenuType.GetChoice}

Procedure MenuType.WriteAsText;
   var a:integer;
   begin
      DisplayTopLine(title);
      setcolor(blue);
      a:= 1;
      while (options[a] <> '') and (a<=MaxNumberOfOptions) do
         begin
         SetTextJustify(LeftText,centerText);
         OutTextXY(XtoPix(SideMargin+0.2),YtoPix(TopMargin+3*LineWidth+1/4*a),
            options[a]);
         inc(a);
         end; {while}
   end; {WriteAsText}

begin
   {none}
end.
