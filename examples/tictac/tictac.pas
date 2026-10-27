{t                              Shannon Behrens
                                6/5/97

This is TicTacToe programmed as practice in OOP.  There are five classes
necessary.  These are defined in unit TicTact and CompUnit.  The are:
            BoardType            PDC
            CompuType

            BoardDrawerType      HIC
            StatusLineType
            MenuType
TicTacToe will hopefully be programmed in graphics mode, and will also
hopefully have a computer opponent.}

Program TicTacToe;

Uses Graph, Crt, MainUnit, Compunit, Ollie, Menu, Dos;

var
   Board : BoardType;
   BoardDrawer : BoardDrawerType;
   StatusLine : StatusLineType;
   MyMenu : MenuType;             {To control Opponent}
   Opponent : OpponentType;
   Oturn : boolean;
   PlayAgain : char;

Procedure Initialize(var Board:BoardType ; var BoardDrawer:BoardDrawerType ;
   var StatusLine:StatusLineType ; var OTurn:Boolean);
   begin
      chdir('a:\TicTac');
      Board.create;
      BoardDrawer.create;
      StatusLine.create;
      Oturn:= true;
   end;

Procedure InitializeGraphics;
   var grDriver: Integer;
       grMode: Integer;
   begin
      grDriver := Detect;
      InitGraph(grDriver, grMode,'a:\tictac');
   end; {Initialize}

Procedure Goodbye;
   {Draw Board, then have Ollie grow in size from the middle of it}
   var Ollie:OllieType;
       Stepper:integer;
   Procedure DrawBoard;
      var Board:BoardType;
          BoardDrawer:BoardDrawerType;
          StatusLine:StatusLineType;
          a:FinishType;
      begin
         Board.create;
         BoardDrawer.create;
         StatusLine.create;
         Board.ExecuteMove('7',true);
         Board.ExecuteMove('3',true);
         Board.ExecuteMove('8',false);
         Board.ExecuteMove('1',false);
         StatusLine.DisplayLine('Goodbye!');
         delay(1500);
      end; {DrawBoard}
   begin
      setGraphMode(GetGraphMode);
      DrawBoard;
      stepper:=1;
      repeat
         Ollie.DrawOllie(6.5,5.55,stepper*0.2);
         delay(200);
         inc(stepper);
      until keypressed or (stepper>35);
      Ollie.blink(6.5,5.55,stepper*0.2);
      closegraph;
   end; {Goodbye}

Procedure Info;
   var Menu:MenuType;
       a:char;
   begin
      Menu.create(1,1);
      Menu.ChangeTitle('About Tic-Tac-Toe');
      Menu.AddOption('Tic-Tac-Toe was Programmed by');
      Menu.AddOption('Shannon "JJ" Behrens in Fayetteville, NC.');
      Menu.AddOption(' ');
      Menu.AddOption('Special thank to my family, especially');
      Menu.AddOption('The love of my life, Gina-Marie Ardizzone.');
      Menu.AddOption(' ');
      Menu.AddOption('Please send questions and comments to:');
      Menu.AddOption('210 Lofton Dr., Fayetteville, NC  28311');
      Menu.WriteAsText;
      a:= readkey;
   end; {info}

Procedure MainMenu(var menu:MenuType ; var opponent:OpponentType);
   begin
         Menu.create(1,1);
         Menu.ChangeTitle('Welcome to Tic-Tac-Toe');
         Menu.AddOption('Player vs. Player');    {1}
         Menu.AddOption('Player vs. Computer (Medium)');  {2}
         Menu.AddOption('Player vs. JJ (Hard)'); {3}
         Menu.AddOption('About Tic-Tac-Toe');    {4}
         Menu.AddOption('Exit');                 {5}
         Case Menu.GetChoice of
            '1'         : Opponent:= player;
            '2'         : Opponent:= computer;
            '3'         : Opponent:= JJ;
            '4'         : begin Info; MainMenu(Menu,Opponent); end;
            '5',chr(27) : Begin GoodBye; halt; end;
         end; {case}
   end; {MainMenu}

Procedure  HumanOpponent(var Board:BoardType ;
   var BoardDrawer:BoardDrawerType ; var StatusLine:StatusLineType ;
   var OTurn:Boolean);
   {Contains one human vs. human game}
   var Finish:FinishType;
   begin
      repeat                   {promt,validate,execute}
         Board.ExecuteMove(StatusLine.prompt('Please choose a space ' +
            StatusLine.WhoseTurn(Oturn) + ':  ')   ,   Oturn);
         Oturn:= not OTurn;
         Finish:= Board.Finish(Oturn);
      until finish<>no;        {Check for all types of finishes, draw line}
         if finish=cat then       {if there's a winner.}
         StatusLine.DisplayBlinkLine('Cat''s Game!')
      else                     {At this point, it's the loser's turn.}
      StatusLine.DisplayBlinkLine('Congratulations ' +
         StatusLine.WhoseTurn(not OTurn) + '!');
   end; {HumanOpponent}

Procedure ComputerOpponent(var Board:BoardType ;
   var BoardDrawer:BoardDrawerType ; var StatusLine:StatusLineType ;
   var OTurn:Boolean ; Opponent:OpponentType);
   var Compu:CompuType;
       Finish:FinishType;
   Procedure FirstMoves(var Compu:CompuType ; var StatusLine:StatusLineType ;
      var Board:BoardType ; Oturn:boolean);
      var FirstMove:char;
      begin
         Compu.create;
         FirstMove:= StatusLine.prompt('Please choose a space:');
         Board.ExecuteMove(FirstMove,Oturn);
         Oturn:= not Oturn;
         StatusLine.DisplayLine('Shhh...I trying to think!');
         Compu.FirstMove(FirstMove,Board);
         Oturn:= not Oturn;
      end; {FirstMoves}
    begin
      FirstMoves(Compu,StatusLine,Board,Oturn);
      repeat                   {promt,validate,execute}
         Board.ExecuteMove(StatusLine.prompt('Please choose a space:')
            ,Oturn);
         Oturn:= not Oturn;
         Finish:= Board.Finish(Oturn);
         if finish = no then   {Check for finish after every move}
            begin
            StatusLine.DisplayLine('Shhh...I am trying to think!');
            Compu.SelectMove(Board,Opponent);
            Oturn:= not Oturn;
            Finish:= Board.Finish(Oturn);
            end; {if}
      until finish<>no;        {Check for all types of finishes, draw line}
      if finish=cat then       {if there's a winner.}
         StatusLine.DisplayBlinkLine('Cat''s Game!')
      else if not OTurn then
         StatusLine.DisplayBlinkLine('Well done, I''m beat!')
      else StatusLine.DisplayBlinkLine('Well played, try again!');
   end; {ComputerOpponent}

begin
      InitializeGraphics;       {Begin Graphics}
   repeat
      MainMenu(MyMenu,opponent);  {Uses Crt}
      SetGraphMode(GetGraphMode);
         repeat
         Initialize(Board,BoardDrawer,StatusLine,OTurn);
         If Opponent=Player then
            HumanOpponent(Board,BoardDrawer,StatusLine,OTurn)
         else ComputerOpponent(Board,BoardDrawer,StatusLine,OTurn,Opponent);
         PlayAgain:= StatusLine.Prompt('Would you like to play again?');
         until (PlayAgain='N') or (PlayAgain=chr(27));
   until 1=2;                   {halt and CloseGraphics are inside main menu}
end.



