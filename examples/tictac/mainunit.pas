{t                              Shannon Behrens
                                7/5/97

Please see tictact2.  This is the main unit, defining all the classes, for
TicTacToe.}

UNIT MainUnit;

INTERFACE

Uses Crt, Graph, Ollie;

Type
   SpacesType = (Nothing,O,X);
   SpacesArray = array['1'..'9'] of SpacesType;
   FinishType = (Win,Cat,No);
   ShortString = string[20];
   BoardType = object                           {PDC}
      spaces : SpacesArray;
      Procedure Create;
      Function ValidMove(choice:char) : Boolean;
      Procedure ExecuteMove(choice:char ; OTurn:boolean);
      Function Finish(Oturn:boolean):FinishType;
      end; {object}
   {Please note that the next two objects use the external functions XtoPix
    and YtoPix}
   BoardDrawerType = object                     {HIC}
      Procedure Create;
      Procedure DrawBoard;
      Procedure DrawNumbers;
      Procedure UpdateBoard(space:char ; OTurn:Boolean);
      Procedure DrawWinningLine(a,b,c:char ; Oturn:Boolean ;
         Spaces:SpacesArray);
      end; {object}
   StatusLineType = object
      Procedure Create;
      Procedure DisplayLine(TheLine:string);
      Procedure DisplayBlinkLine(TheLine:string);
      Function Prompt(TheLine:string):char;
      Function WhoseTurn(OTurn:boolean):ShortString;
      end; {object}

IMPLEMENTATION
   Procedure BoardType.Create;
      var a : char;
      begin
         for a:= '1' to '9' do
            Spaces[a]:= Nothing;
      end; {BoardType.Create}

   Function BoardType.ValidMove(choice:char):Boolean;
      begin                     {to circumvent array bounds checking}
         if (choice>='1') and (choice<='9') then
            ValidMove:= spaces[choice]= nothing
         else ValidMove:= false;
      end; {BoardType.ValidMove}

   Procedure BoardType.ExecuteMove(choice:char ; OTurn:boolean);
      {Determine if move is valid.  If not, get one that is.  Enter it into
       the spaces array, and draw it}
      var StatusLine:StatusLineType;
          BoardDrawer:BoardDrawerType;
      begin
         if not ValidMove(choice) then
            repeat
               StatusLine.Create;
               choice:= StatusLine.Prompt(chr(7) +
                  'Invalid Move, please try again:  ');
            until ValidMove(choice);
         if OTurn then spaces[choice]:= O
            else spaces[choice]:= X;
         BoardDrawer.UpdateBoard(choice,OTurn);
      end; {BoardType.ExecuteMove}

   Function BoardType.Finish(Oturn:boolean):FinishType;
      Function Winner : Boolean;
         begin
            Winner:=
            ((spaces['1']=spaces['2']) and (spaces['2']=spaces['3'])
             and (spaces['3']<>nothing)) or
            ((spaces['4']=spaces['5']) and (spaces['5']=spaces['6'])
             and (spaces['6']<>nothing)) or
            ((spaces['7']=spaces['8']) and (spaces['8']=spaces['9'])
             and (spaces['9']<>nothing)) or
            ((spaces['1']=spaces['4']) and (spaces['4']=spaces['7'])
             and (spaces['7']<>nothing)) or
            ((spaces['2']=spaces['5']) and (spaces['5']=spaces['8'])
             and (spaces['8']<>nothing)) or
            ((spaces['3']=spaces['6']) and (spaces['6']=spaces['9'])
             and (spaces['9']<>nothing)) or
            ((spaces['1']=spaces['5']) and (spaces['5']=spaces['9'])
             and (spaces['9']<>nothing)) or
            ((spaces['7']=spaces['5']) and (spaces['5']=spaces['3'])
             and (spaces['3']<>nothing));
         end; {Winner}
      Procedure DealWithWinner(Oturn:boolean);
         var Board:BoardDrawerType;
             Temp:SpacesArray;
         begin
            Temp:= spaces;              {Send them a copy}
            if ((spaces['1']=spaces['2']) and (spaces['2']=spaces['3'])
               and (spaces['3']<>nothing)) then
               Board.DrawWinningLine('1','2','3',Oturn,temp)
            else if ((spaces['4']=spaces['5']) and (spaces['5']=spaces['6'])
               and (spaces['6']<>nothing)) then
               Board.DrawWinningLine('4','5','6',Oturn,temp)
            else if ((spaces['7']=spaces['8']) and (spaces['8']=spaces['9'])
               and (spaces['9']<>nothing)) then
               Board.DrawWinningLine('7','8','9',Oturn,temp)
            else if ((spaces['1']=spaces['4']) and (spaces['4']=spaces['7'])
               and (spaces['7']<>nothing)) then
               Board.DrawWinningLine('1','4','7',Oturn,temp)
            else if ((spaces['2']=spaces['5']) and (spaces['5']=spaces['8'])
               and (spaces['8']<>nothing)) then
               Board.DrawWinningLine('2','5','8',Oturn,temp)
            else if ((spaces['3']=spaces['6']) and (spaces['6']=spaces['9'])
               and (spaces['9']<>nothing)) then
               Board.DrawWinningLine('3','6','9',Oturn,temp)
            else if ((spaces['1']=spaces['5']) and (spaces['5']=spaces['9'])
               and (spaces['9']<>nothing)) then
               Board.DrawWinningLine('1','5','9',Oturn,temp)
            else if ((spaces['7']=spaces['5']) and (spaces['5']=spaces['3'])
               and (spaces['3']<>nothing)) then
               Board.DrawWinningLine('3','5','7',Oturn,temp);
         end; {DealWithWinner}
      Function CheckForCatsGame : Boolean;
         var Return:boolean;
             a:char;
         begin
            Return:= true;
            for a:= '1' to '9' do
               if spaces[a]=Nothing then Return:= false;
            CheckForCatsGame:= Return;
         end; {CheckForCatsGame}
      begin {BoardType.Finish}
         if Winner then
            begin finish:= win; DealWithWinner(Oturn); end
         else if CheckForCatsGame then finish:= cat
         else finish:= no;
      end; {BoardType.Finish}

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

   Procedure BoardDrawerType.Create;
      begin
         ClearViewPort;
         DrawBoard;
         DrawNumbers;
      end; {BoardDrawerType.Create}

   Procedure BoardDrawerType.DrawBoard;
      begin
         Setcolor(blue);
         Line(XtoPix(1),YtoPix(4),XtoPix(4),YtoPix(4));
         Line(XtoPix(1),YtoPix(6),XtoPix(4),YtoPix(6));
         Line(XtoPix(2),YtoPix(2),XtoPix(2),YtoPix(8));
         Line(XtoPix(3),YtoPix(2),XtoPix(3),YtoPix(8));
      end; {BoardDrawerType.DrawBoard}

   Procedure BoardDrawerType.DrawNumbers;
      {Numbers should be in the lower left hand corner of each of the nine
       spaces.  They should be numbers in the order of a standard keypad}
      Const LittleX =  0.07;            {place nums slightly above line}
            LittleY = -0.2;
      begin
         Setcolor(yellow);
         OutTextXY(XtoPix(1+LittleX),YtoPix(4+LittleY), '7');
         OutTextXY(XtoPix(2+LittleX),YtoPix(4+LittleY), '8');
         OutTextXY(XtoPix(3+LittleX),YtoPix(4+LittleY), '9');
         OutTextXY(XtoPix(1+LittleX),YtoPix(6+LittleY), '4');
         OutTextXY(XtoPix(2+LittleX),YtoPix(6+LittleY), '5');
         OutTextXY(XtoPix(3+LittleX),YtoPix(6+LittleY), '6');
         OutTextXY(XtoPix(1+LittleX),YtoPix(8+LittleY), '1');
         OutTextXY(XtoPix(2+LittleX),YtoPix(8+LittleY), '2');
         OutTextXY(XtoPix(3+LittleX),YtoPix(8+LittleY), '3');
      end; {BoardDrawerType.DrawNumbers}

   Procedure BoardDrawerType.UpdateBoard(space:char ; Oturn:boolean);
      var a,b:real;
      Procedure DetermineCenter(space:char ; var a,b:real);
         {Will return unit type, not pixel type}
         begin
            Case space of
               '1' : begin a:= 1.5; b:= 7; end;
               '2' : begin a:= 2.5; b:= 7; end;
               '3' : begin a:= 3.5; b:= 7; end;
               '4' : begin a:= 1.5; b:= 5; end;
               '5' : begin a:= 2.5; b:= 5; end;
               '6' : begin a:= 3.5; b:= 5; end;
               '7' : begin a:= 1.5; b:= 3; end;
               '8' : begin a:= 2.5; b:= 3; end;
               '9' : begin a:= 3.5; b:= 3; end;
            end; {case}
         end; {DetermineCenter}
      Procedure DrawO(a,b:real);
         const RadiusInUnits = 0.3;
         begin
            circle(XtoPix(a),YtoPix(b),XtoPix(RadiusInUnits));
         end; {DrawO}
      Procedure DrawX(a,b:real);
         {Use center to draw \, then /}
         Const XSpacer = 0.35;
               YSpacer = 0.7;
         begin
            Line(XtoPix(a-XSpacer),YtoPix(b-YSpacer),
               XtoPix(a+XSpacer),YtoPix(b+YSpacer));
            Line(XtoPix(a+XSpacer),YtoPix(b-YSpacer),
               XtoPix(a-XSpacer),YtoPix(b+YSpacer));
         end; {DrawX}
      begin {BoardDrawerType.UpdateBoard}
         SetColor(yellow);
         DetermineCenter(space,a,b);
         if Oturn then DrawO(a,b)
            else DrawX(a,b);
      end; {BoardDrawerType.UpdateBoard}

   Procedure BoardDrawerType.DrawWinningLine(a,b,c:char ; Oturn:boolean ;
      spaces:SpacesArray);
      var x1,y1,x2,y2:real;
      Procedure DetermineEndpoints(a,b,c:char ; var x1,y1,x2,y2:real);
         {returns the endpoints of the line going through the winning spaces.
          Note the order in which a,b, and c are sent from Board.DealWithWinners.
          Also notice that there are only 8 possible winning paths here}
         begin
            case a of
               '1' : Case c of
                     '3' :      begin x1:=1 ; y1:=7 ; x2:=4 ; y2:=7 ; end;
                     '7' :      begin x1:=1.5 ; y1:=8 ; x2:=1.5 ; y2:=2 ; end;
                     '9' :      begin x1:=1 ; y1:=8 ; x2:=4 ; y2:=2 ; end;
                     end; {Case}
              '2' :             begin x1:=2.5 ; y1:=8 ; x2:=2.5 ; y2:=2 ; end;
              '3' : Case c of
                     '7' :      begin x1:=4 ; y1:=8 ; x2:=1 ; y2:=2 ; end;
                     '9' :      begin x1:=3.5 ; y1:=8 ; x2:=3.5 ; y2:=2 ; end;
                    end; {case}
              '4' :             begin x1:=1 ; y1:=5 ; x2:=4 ; y2:=5 ; end;
              '7' :             begin x1:=1 ; y1:=3 ; x2:=4 ; y2:=3 ; end;
            end; {case}
         end; {DetermineEndpoints}
      Procedure DrawOllie(a:char);
         {Draw Ollie's on top of the old circles.  Numbers come from
          calculations}
         Const OllieRadius = 1;
         Var Ollie:OllieType;
         begin
            Case a of
               '1' : Ollie.DrawOllie(3.9,7.77,OllieRadius);
               '2' : Ollie.DrawOllie(6.5,7.77,OllieRadius);
               '3' : Ollie.DrawOllie(9.1,7.77,OllieRadius);
               '4' : Ollie.DrawOllie(3.9,5.55,OllieRadius);
               '5' : Ollie.DrawOllie(6.5,5.55,OllieRadius);
               '6' : Ollie.DrawOllie(9.1,5.55,OllieRadius);
               '7' : Ollie.DrawOllie(3.9,3.33,OllieRadius);
               '8' : Ollie.DrawOllie(6.5,3.33,OllieRadius);
               '9' : Ollie.DrawOllie(9.1,3.33,OllieRadius);
            end; {Case}
         end; {DrawOllie}
      Procedure DrawFrowners(spaces:SpacesArray);
         {Draw Ollie's frown on top of the old circles.  Numbers come from
          calculations}
         Const OllieRadius = 1;
         Var Ollie:OllieType;
             a:char;
         begin
            For a:= '1' to '9' do
               if (spaces[a]=O) then
                  Case a of
                    '1' : Ollie.Frown(3.9,7.77,OllieRadius);
                    '2' : Ollie.Frown(6.5,7.77,OllieRadius);
                    '3' : Ollie.Frown(9.1,7.77,OllieRadius);
                    '4' : Ollie.Frown(3.9,5.55,OllieRadius);
                    '5' : Ollie.Frown(6.5,5.55,OllieRadius);
                    '6' : Ollie.Frown(9.1,5.55,OllieRadius);
                    '7' : Ollie.Frown(3.9,3.33,OllieRadius);
                    '8' : Ollie.Frown(6.5,3.33,OllieRadius);
                    '9' : Ollie.Frown(9.1,3.33,OllieRadius);
                  end; {Case}
         end; {DrawOllie}
      Procedure BigXes(spaces:SpacesArray);
         var a:char;
         begin
            for a:= '1' to '9' do
               if spaces[a] = X then
               begin
                  SetLineStyle(SolidLn,0,ThickWidth);
                  UpdateBoard(a,false);
                  SetLineStyle(SolidLn,0,NormWidth);
               end; {if}
         end; {BigExes}
      begin
         if not Oturn then        {if O is winning}
            begin DrawOllie(a);DrawOllie(b);DrawOllie(c) end
         else begin DrawFrowners(Spaces); BigXes(Spaces); end;
         DetermineEndpoints(a,b,c,x1,y1,x2,y2);
         setcolor(green);
         Line(XtoPix(x1),YtoPix(y1),XtoPix(x2),YtoPix(y2));
      end; {BoardDrawerType.DrawWinningLine}

   Procedure StatusLineType.Create;
      Const Margin=0.25;
            LineWidth=0.25;
      var a:integer;
          OldFillPattern:FillPatternType;
      begin
         GetFillPattern(OldFillPattern);
         SetColor(blue);
         SetFillPattern(OldFillPattern,blue);
         bar(XtoPix(0+margin),YtoPix(0+Margin),
            XtoPix(5-margin),YtoPix(0+Margin+LineWidth));
      end; {StatusLineType.Create}

   Procedure StatusLineType.DisplayLine(TheLine:string);
      Const VerticalCenter=0.375;
            HorizontalCenter=2.5;
      begin
         Create;                {erases current line}
         SetColor(yellow);
         SetTextJustify(centerText,centerText);
         OutTextXY(XToPix(HorizontalCenter),YToPix(VerticalCenter),TheLine);
      end; {StatusLineType.DisplayLine}

   Procedure StatusLineType.DisplayBlinkLine(TheLine:string);
      Const VerticalCenter=0.375;
            HorizontalCenter=2.5;
            DelayRate=200;       {2.5 blinks per second}
      var BufferChar:char;
      begin
         SetTextJustify(centerText,centerText);
         repeat
            Create;             {erases current line}
            Delay(DelayRate);
            SetColor(yellow);
            OutTextXY(XToPix(HorizontalCenter),
               YToPix(VerticalCenter),TheLine);
            Delay(DelayRate);
         until keypressed;
         BufferChar:= readkey;  {clear the readkey buffer}
      end; {StatusLineType.DisplayBlinkLine}

   Function StatusLineType.Prompt(TheLine:string):char;
      begin
         DisplayLine(TheLine);
         Prompt:= Upcase(readkey);
      end; {StatusLineType.Prompt}

   Function StatusLineType.WhoseTurn(OTurn:boolean):ShortString;
      begin
         if OTurn then WhoseTurn:= 'Player One'
            else WhoseTurn:= 'Player Two';
      end; {StatusLineType.WhoseTurn}

begin
  {no initializations}
end.