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

program TicTacToe;

uses Graph, Crt, MainUnit, CompUnit, Ollie, Menu, Dos;

var
   Board       : BoardType;
   BoardDrawer : BoardDrawerType;
   StatusLine  : StatusLineType;
   MyMenu      : MenuType;        {To control Opponent}
   Opponent    : OpponentType;
   Oturn       : boolean;
   playagain   : char;

procedure Initialize(var Board: BoardType; var BoardDrawer: BoardDrawerType;
   var StatusLine: StatusLineType; var Oturn: boolean);
   begin
      chdir('a:\TicTac');
      Board.Create;
      BoardDrawer.Create;
      StatusLine.Create;
      Oturn := true;
   end;

procedure InitializeGraphics;
   var grDriver: integer;
      grMode: integer;
   begin
      grDriver := Detect;
      InitGraph(grDriver, grMode, 'a:\tictac');
   end; {Initialize}

procedure Goodbye;
   {Draw Board, then have Ollie grow in size from the middle of it}
   var Ollie: OllieType;
      stepper: integer;
   procedure DrawBoard;
      var Board: BoardType;
         BoardDrawer: BoardDrawerType;
         StatusLine: StatusLineType;
         a: FinishType;
      begin
         Board.Create;
         BoardDrawer.Create;
         StatusLine.Create;
         Board.ExecuteMove('7', true);
         Board.ExecuteMove('3', true);
         Board.ExecuteMove('8', false);
         Board.ExecuteMove('1', false);
         StatusLine.DisplayLine('Goodbye!');
         delay(1500);
      end; {DrawBoard}
   begin
      setGraphMode(GetGraphMode);
      DrawBoard;
      stepper := 1;
      repeat
         Ollie.DrawOllie(6.5, 5.55, stepper*0.2);
         delay(200);
         inc(stepper);
      until keypressed or (stepper>35);
      Ollie.blink(6.5, 5.55, stepper*0.2);
      CloseGraph;
   end; {Goodbye}

procedure Info;
   var Menu: MenuType;
      a: char;
   begin
      Menu.Create(1, 1);
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
      a := readkey;
   end; {info}

procedure MainMenu(var Menu: MenuType; var Opponent: OpponentType);
   begin
      Menu.Create(1, 1);
      Menu.ChangeTitle('Welcome to Tic-Tac-Toe');
      Menu.AddOption('Player vs. Player');    {1}
      Menu.AddOption('Player vs. Computer (Medium)');  {2}
      Menu.AddOption('Player vs. JJ (Hard)'); {3}
      Menu.AddOption('About Tic-Tac-Toe');    {4}
      Menu.AddOption('Exit');                 {5}
      case Menu.GetChoice of
         '1'          : Opponent := player;
         '2'          : Opponent := computer;
         '3'          : Opponent := JJ;
         '4'          : begin Info; MainMenu(Menu, Opponent); end;
         '5', chr(27) : begin Goodbye; halt; end;
      end; {case}
   end; {MainMenu}

procedure  HumanOpponent(var Board: BoardType;
   var BoardDrawer: BoardDrawerType; var StatusLine: StatusLineType;
   var Oturn: boolean);
   {Contains one human vs. human game}
   var Finish: FinishType;
   begin
      repeat                   {promt,validate,execute}
         Board.ExecuteMove(StatusLine.Prompt('Please choose a space ' +
            StatusLine.WhoseTurn(Oturn) + ':  '), Oturn);
         Oturn := not Oturn;
         Finish := Board.Finish(Oturn);
      until Finish <> no;      {Check for all types of finishes, draw line}
      if Finish = cat then     {if there's a winner.}
         StatusLine.DisplayBlinkLine('Cat''s Game!')
      else                     {At this point, it's the loser's turn.}
         StatusLine.DisplayBlinkLine('Congratulations ' +
            StatusLine.WhoseTurn(not Oturn) + '!');
   end; {HumanOpponent}

procedure ComputerOpponent(var Board: BoardType;
   var BoardDrawer: BoardDrawerType; var StatusLine: StatusLineType;
   var Oturn: boolean; Opponent: OpponentType);
   var Compu: CompuType;
      Finish: FinishType;
   procedure FirstMoves(var Compu: CompuType; var StatusLine: StatusLineType;
      var Board: BoardType; Oturn: boolean);
      var FirstMove: char;
      begin
         Compu.Create;
         FirstMove := StatusLine.Prompt('Please choose a space:');
         Board.ExecuteMove(FirstMove, Oturn);
         Oturn := not Oturn;
         StatusLine.DisplayLine('Shhh...I trying to think!');
         Compu.FirstMove(FirstMove, Board);
         Oturn := not Oturn;
      end; {FirstMoves}
   begin
      FirstMoves(Compu, StatusLine, Board, Oturn);
      repeat                   {promt,validate,execute}
         Board.ExecuteMove(StatusLine.Prompt('Please choose a space:')
            , Oturn);
         Oturn := not Oturn;
         Finish := Board.Finish(Oturn);
         if Finish = no then   {Check for finish after every move}
         begin
            StatusLine.DisplayLine('Shhh...I am trying to think!');
            Compu.SelectMove(Board, Opponent);
            Oturn := not Oturn;
            Finish := Board.Finish(Oturn);
         end; {if}
      until Finish <> no;      {Check for all types of finishes, draw line}
      if Finish = cat then     {if there's a winner.}
         StatusLine.DisplayBlinkLine('Cat''s Game!')
      else if not Oturn then
         StatusLine.DisplayBlinkLine('Well done, I''m beat!')
      else StatusLine.DisplayBlinkLine('Well played, try again!');
   end; {ComputerOpponent}

begin
   InitializeGraphics;       {Begin Graphics}
   repeat
      MainMenu(MyMenu, Opponent); {Uses Crt}
      setGraphMode(GetGraphMode);
      repeat
         Initialize(Board, BoardDrawer, StatusLine, Oturn);
         if Opponent = player then
            HumanOpponent(Board, BoardDrawer, StatusLine, Oturn)
         else ComputerOpponent(Board, BoardDrawer, StatusLine, Oturn, Opponent);
         playagain := StatusLine.Prompt('Would you like to play again?');
      until (playagain = 'N') or (playagain = chr(27));
   until 1 = 2;                 {halt and CloseGraphics are inside main menu}
end.
