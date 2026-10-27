{t                              Shannon Behrens
                                7/18/97

Clock - This is a dos based clock made to look like the one in windows, but
this one will hopefully run counter-clockwise.  This is a descendent of menu.
Note, the scale 0..5 X and 0..9 Y is used.  Requirement:  must be in graph
mode.}

Unit Clock;

INTERFACE
   Uses Menu,Graph,Crt,Dos;
   Const
      Days : array [0..6] of String[9] = ('Sunday','Monday','Tuesday',
         'Wednesday','Thursday','Friday','Saturday');
   Type
      TimeType = record Hours,Minutes,Seconds,Sec100s:word; end; {record}
      DateType = record Year,Month,Day,DayOfWeek:word; end; {record}
      CenterType = record X,Y:real; end; {record}
      ClockType = object(MenuType)
         {private}
            Time:TimeType;
            Date:DateType;
            Center:CenterType;
            Radius:Real;                {0..9 from YtoPix}
            Procedure DisplayMinutes(TempMin:word ; Color:word);
            Procedure DisplaySeconds(TempSec:word ; Color:word);
            Procedure DisplayHours(TempHour,TempMin:word ; Color:word);
         {public}
            Procedure Create(margin:real);
            Procedure Update;
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

Procedure ClockType.Create(margin:real);
   var MyLine:string;
   Procedure CreateWindow;
      var OldFillPattern:FillPatternType;
      begin
         setbkcolor(white);
         ClearViewPort;
         GetFillPattern(OldFillPattern);
         SetFillPattern(OldFillPattern,blue);                       {Top Bar}
         bar(XtoPix(0+Sidemargin*0.99),YtoPix(0+TopMargin),
            XtoPix(5-Sidemargin*0.99),YtoPix(0+TopMargin+LineWidth));
         SetFillPattern(OldFillPattern,LightGray);
         SetColor(blue);
         Bar(XtoPix(0+Sidemargin),YtoPix(0+TopMargin+LineWidth),    {Gray area}
            XtoPix(5-SideMargin),YtoPix(9-3/2*TopMargin));
         SetLineStyle(SolidLn,0,ThickWidth);
         rectangle(XtoPix(0+Sidemargin),YtoPix(0+TopMargin+LineWidth),
            XtoPix(5-SideMargin),YtoPix(9-3/2*TopMargin));
         SetLineStyle(SolidLn,0,NormWidth);
      end; {CreateWindow}
   Procedure InitVars(Margin:Real ; var TopMargin,SideMargin:Real ;
      var Title:string40 ; var Options:Optionstype ; var Center:Centertype ;
      var Radius:Real ; var Time:Timetype ; var Date:DateType);
      {Initializes all variables}
      var a:integer;
      begin
         TopMargin:= Margin; SideMargin:= Margin*7/9;
         Title:= '';
         for a:= 1 to MaxNumberOfOptions do
            Options[a]:= '';
         Center.Y:= ((9-3/2*TopMargin)+(TopMargin+LineWidth))/2;
         Center.X:= 2.5;
         Radius:= (Center.Y-TopMargin-LineWidth-0.75*LineWidth);
         GetDate(Date.Year,Date.Month,Date.Day,Date.DayOfWeek);
         GetTime(Time.Hours,Time.Minutes,Time.Seconds,Time.Sec100s);
      end; {InitVars}
   Procedure DrawSquares(Center:CenterType ; Radius:Real);
      var a:integer;
      Procedure PutBox(X,Y:integer);
         {Receives an X and Y coordinate and draws a small box around the
          coordinate (in pixels)}
         const BoxWidth = 7;                             {in pixels}
               Boxheight = 7;
         var OldFillPattern:FillPatternType;
         begin
            GetFillPattern(OldFillPattern);
            SetFillPattern(OldFillPattern,White);        {Background}
            Bar(X-BoxWidth div 2 - 1 , Y-BoxHeight div 2 - 1 ,
               X+BoxWidth div 2 , Y+BoxHeight div 2);
            SetFillPattern(OldFillPattern,Blue);
            Bar(X-BoxWidth div 2 , Y-BoxHeight div 2 ,   {Foreground}
               X+BoxWidth div 2 , Y+BoxHeight div 2);
         end; {Putbox}
      begin                         {Draw small dots}
         for a:= 0 to 59 do
            PutPixel(XtoPix(Center.X)+YtoPix(Radius*cos(a*pi/30)) ,
               YtoPix(Center.Y-Radius*sin(a*pi/30)),White);
         for a:= 0 to 59 do
            PutPixel(XtoPix(Center.X)+YtoPix(Radius*cos(a*pi/30)) +1 ,
               YtoPix(Center.Y-Radius*sin(a*pi/30)) +1 , DarkGray);
         for a:= 0 to 11 do         {Draw small boxes}
            PutBox(XtoPix(Center.X)+YtoPix(Radius*cos(a*pi/6)) ,
               YtoPix(Center.Y-Radius*sin(a*pi/6)));
      end; {DrawSquares}
   function IntToStr(I: Longint): String;
      {Convert any integer type to a string}
      var
         S: string[11];
      begin
         Str(I, S);
         IntToStr := S;
      end; {IntToStr}
   Procedure DisplayBottomLine;
      begin
         SetTextJustify(RightText,CenterText);
         SetColor(White);
         OutTextXy(XtoPix(5-SideMargin)-1,YToPix(9-3/2*TopMargin-0.2)-1,
            'Exit, Info ');
         OutTextXy(XtoPix(5-SideMargin)-1,YToPix(9-3/2*TopMargin-0.2)+1-1,
            '_     _    ');
         SetColor(DarkGray);
         OutTextXy(XtoPix(5-SideMargin),YToPix(9-3/2*TopMargin-0.2) ,
            'Exit, Info ');
         OutTextXy(XtoPix(5-SideMargin),YToPix(9-3/2*TopMargin-0.2)+1 ,
            '_     _    ');
         SetTextJustify(CenterText,CenterText);
      end; {DisplayBottomLine}
   begin {ClockType.Create}
      InitVars(Margin,TopMargin,SideMargin,Title,Options,Center,Radius,Time,
         Date);
      CreateWindow;
      DrawSquares(center,radius);
      MyLine:= 'Vector Clock - ' + days[Date.DayOfWeek] + ' ' +
         IntToStr(Date.month) + '/' + IntToStr(Date.Day) +
         '/' + IntToStr(Date.Year);
      DisplayTopLine(MyLine);
      DisplayBottomLine;
      DisplayMinutes(Time.Minutes,DarkGray);
      DisplaySeconds(Time.Seconds,Red);
      DisplayHours(Time.Hours,Time.Minutes,DarkGray);
   end; {ClockType.Create}

Procedure ClockType.DisplaySeconds(TempSec:word ; Color:word);
   var ArmLength:real;
       Theta:real;
   begin
      SetColor(Color);
      ArmLength:= 0.8*Radius;
      Theta:= (TempSec/60)*(2*pi)+pi/2; {CountClockWise starting at 12:00}
      Line(XtoPix(Center.X),YtoPix(Center.Y) ,
         YtoPix(ArmLength*cos(theta)) + XtoPix(Center.X) ,
         YtoPix(Center.Y-ArmLength*sin(theta)));
   end; {ClockType.DisplaySeconds}

Procedure ClockType.DisplayMinutes(TempMin:word ; Color:word);
   var ArmLength:real;
       ArrowLength:real;
       Theta:real;
       EndPoint:record x,y:integer; end;
   begin
      SetColor(color);
      ArmLength:= 0.8*Radius;
      ArrowLength:= 0.1*ArmLength;
      Theta:= (TempMin/60)*(2*pi)+pi/2; {CountClockWise starting at 12:00}
      EndPoint.X:= YtoPix(ArmLength*cos(theta)) + XtoPix(Center.X);
      EndPoint.Y:= YtoPix(Center.Y-ArmLength*sin(theta));
      Line(XtoPix(Center.X),YtoPix(Center.Y),EndPoint.X,EndPoint.Y);
      Line(EndPoint.X,EndPoint.Y,YtoPix(ArrowLength*cos(theta-5/6*pi))+
         EndPoint.X,EndPoint.Y-YtoPix(ArrowLength*sin(theta-5/6*pi)));
      Line(EndPoint.X,EndPoint.Y,YtoPix(ArrowLength*cos(theta+5/6*pi))+
         EndPoint.X,EndPoint.Y-YtoPix(ArrowLength*sin(theta+5/6*pi)));
   end; {ClockType.DisplayMinutes}

Procedure ClockType.DisplayHours(TempHour,TempMin:word ; Color:word);
   {Draw a vector looking clock arm.  Main line, then the arrows}
   var ArmLength:real;
       ArrowLength:real;
       Theta:real;
       EndPoint:record x,y:integer; end;
   begin
      SetColor(color);
      ArmLength:= 0.6*Radius;
      ArrowLength:= 0.17*ArmLength;
      Theta:= (TempHour/12)*(2*pi)+pi/2+
         (TempMin/60 * pi/6);            {Not on number exactly}
      EndPoint.X:= YtoPix(ArmLength*cos(theta)) + XtoPix(Center.X);
      EndPoint.Y:= YtoPix(Center.Y-ArmLength*sin(theta));
      Line(XtoPix(Center.X),YtoPix(Center.Y),EndPoint.X,EndPoint.Y);
      Line(EndPoint.X,EndPoint.Y,YtoPix(ArrowLength*cos(theta-3/4*pi))+
         EndPoint.X,EndPoint.Y-YtoPix(ArrowLength*sin(theta-3/4*pi)));
      Line(EndPoint.X,EndPoint.Y,YtoPix(ArrowLength*cos(theta+3/4*pi))+
         EndPoint.X,EndPoint.Y-YtoPix(ArrowLength*sin(theta+3/4*pi)));
   end; {ClockType.DisplayHours}

Procedure ClockType.Update;
   Const TickTime = 0;
         OperationTime = 0;            {Time the compu naturally takes}
   var TempDate:DateType;
       MyLine:String;
   function IntToStr(I: Longint): String;
      {Convert any integer type to a string}
      var
         S: string[11];
      begin
         Str(I, S);
         IntToStr := S;
      end; {IntToStr}
   begin
      Delay(1000-TickTime-OperationTime);
      Sound(1000);                        {tick}
      Delay(TickTime);
      NoSound;
      DisplayMinutes(Time.Minutes,LightGray);
      DisplayHours(Time.Hours,Time.Minutes,LightGray);
      DisplaySeconds(Time.Seconds,LightGray);
      GetTime(Time.Hours,Time.Minutes,Time.Seconds,Time.Sec100s);
      DisplayMinutes(Time.Minutes,DarkGray);
      DisplayHours(Time.Hours,Time.Minutes,DarkGray);
      DisplaySeconds(Time.Seconds,Red);
      TempDate:= Date;
      GetDate(Date.Year,Date.Month,Date.Day,Date.DayOfWeek);
      If TempDate.Day<>Date.Day then
         begin
         MyLine:= 'Vector Clock - ' + days[Date.DayOfWeek] + ' ' +
            IntToStr(Date.month) + '/' + IntToStr(Date.Day) +
            '/' + IntToStr(Date.Year);
         DisplayTopLine(MyLine);
         end; {if}
   end; {ClockType.Update}

begin
   {none}
end.