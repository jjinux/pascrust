{t                              Shannon Behrens
                                6/6/97

This is CompUnit, the PDC class responsible for computer strategy in
TicTacToe.  Please see TicTact2.pas for more information.

This unit is based on the following information/assumptions:

Correctly played, these are the outcomes (for first and second move) where
L represents a loss for the computer.  Please note that the computer always
takes the second move:

 cs:L cm:cat cc:L                         7c 8s 9c
 ms:L mc:cat                              4s 5m 6s
 sm:cat ss:L sc:cat                       1c 2m 6c

 Strategy:
    Player always takes the first move
    Case Move of
       Side : take opposite side;
       corner : take middle;
       middle : take corner;
    MidGame strategies:
       If you can do a checkmate, do it;
       If the player is about to do a checkmake, block it;
       If the setup is nothing but a diagnol line, pick a side;
       Move to the spot where you can get the most checks (probably one);

The goal is to create a strategy that is impossible to beat, and for a good
player will always lead to a catsgame.}

Unit CompUnit;

INTERFACE
Uses Crt,MainUnit;
Const ComputerDelayTime=(1000);
Type
   OpponentType = (Player,Computer,JJ);
   CompuType = object                   {public}
      Procedure Create;
      Procedure FirstMove(PlayersMove:char ; var Board:BoardType);
      Procedure SelectMove(var board:boardType ; Opponent:OpponentType);
                                        {protected}
      Function TryCheckmate(var board:BoardType):boolean;
      Function BlockCheckmate(var board:BoardType):boolean;
      Function BlockDiagnolSituation(var board:BoardType):boolean;
      Procedure GetGoodSpot(var board:BoardType ; Opponent:OpponentType);
      end; {object}

   HypotheticalBoard = Object(BoardType){descendent of board}
      {Does everything that board does, without actually changing the graphics
       screen.  Thus when a HypotheticalBoard is created, it is hypothetical}
      Procedure HypExeMove(choice:char ; OTurn:boolean);
      Function HypFinish : FinishType;
      end; {object}

IMPLEMENTATION
   Procedure CompuType.Create;
      begin {none}
      end; {CompuType.Create}

   Function CompuType.TryCheckmate(var board:BoardType):boolean;
      {Checks to see if there is anyway of checkmating the opponent.  It there
       is, execute that way and return true.  Otherwise, return false}
      var a:char;
          tempBoard:HypotheticalBoard;
          tempReturn:boolean;
      begin
         tempReturn:= false;
         a:= '1';
         while (a<='9') and not TempReturn do
            begin
            if Board.ValidMove(a) then
               begin
               tempBoard.spaces := Board.spaces;
               tempBoard.HypExeMove(a,false);
               if tempBoard.HypFinish=win then
                  begin board.ExecuteMove(a,false); TempReturn:= true; end;
               end; {if}
            inc(a);
            end; {while}
         TryCheckmate:= tempReturn;
      end; {CompuType.TryCheckmate}

   Function CompuType.BlockCheckmate(var board:BoardType):boolean;
      {Check to see if there is anyway that the player can get checkmate on
       his next move.  If there is, block it and return true.  If not,
       return false.}
      var a:char;
          tempBoard:HypotheticalBoard;
          tempReturn:boolean;
      begin
         tempReturn:= false;
         a:= '1';
         while (a<='9') and not TempReturn do
            begin
            if Board.ValidMove(a) then
               begin                        {true - pretend you're the player}
               tempBoard.spaces:= Board.spaces;
               tempBoard.HypExeMove(a,true);
               if tempBoard.HypFinish=win then
                  begin Board.ExecuteMove(a,false); TempReturn:= true; end;
               end; {if}                {block it}
            inc(a);
            end; {while}
         BlockCheckmate:= tempReturn;
      end; {CompuType.BlockCheckmate}

   Function CompuType.BlockDiagnolSituation(var board:BoardType):boolean;
      {At the interface level, we must find out if we are in a situation
       in which spots (753) or (951) are taken.  It they are, pick a side
       and return true.  Otherwise, return false.}
      {Friend of Board}
      Procedure BlockIt(var Board:BoardType);
         begin
            case random(4) of      {pick a side}
              0 : Board.ExecuteMove('8',false);
              1 : Board.ExecuteMove('4',false);
              2 : Board.ExecuteMove('6',false);
              3 : Board.ExecuteMove('2',false);
            end; {case}
         end; {BlockIt}
      begin
         if (board.spaces['1']=O) and (board.spaces['5']=X) and
            (board.spaces['9']=O) then
            begin BlockDiagnolSituation:= true; BlockIt(board); end
         else if (board.spaces['7']=O) and (board.spaces['5']=X) and
            (board.spaces['3']=O) then
            begin BlockDiagnolSituation:= true; BlockIt(board); end
         else BlockDiagnolSituation:= false;
      end; {CompuType.BlockDiagnolSituation}

   Procedure CompuType.GetGoodSpot(var board:BoardType ;
      Opponent:OpponentType);
      {This uses a points system in order to pick the best spot.  This is the
       last priority, so it is not a function.  If Opponent = computer, then
       BlockDoubleCheck is basically deactivated to make it easier for the
       player.}
      Type SpotsType = array['1'..'9'] of integer;
      var  a:char;
           spots:SpotsType;
      Function AccountForValidity(Board:BoardType ; Move:Char):integer;
         {All valid spots are given a 1, invalid ones are given nothing}
         begin
            if Board.ValidMove(Move) = true then
               AccountForValidity:= 1
            else AccountForValidity:= 0;
         end; {AccountForValidity}
      Function CheckOutSpot(board:boardType ; a:char ; Oturn:boolean):integer;
         {Given a player and his move, this F returns the amount of check
          situations for him on the board.}
         var return:integer;
             temp,temp2:HypotheticalBoard;
             b:char;                    {The next hypothetical comp move}
         begin
            temp.spaces:= board.spaces; {The first hypothetical board}
            temp.HypExeMove(a,Oturn);
            return:= 0;
            for b:= '1' to '9' do
               begin
               if temp.validmove(b) then
                  begin
                  temp2:= temp;         {The next step hypothetical board}
                  temp2.HypExeMove(b,Oturn);
                  if temp2.HypFinish=win then inc(return);
                  end; {if}
               end; {for}
            CheckOutSpot:= return;
         end; {CheckOutSpot}
      Function BlockDoubleCheck(Board:BoardType ; Move:Char):integer;
         {Returns two points if a spot has the potential of being a double
          check for the opponent}
         begin
            if CheckOutSpot(board,move,true)=2 then
               BlockDoubleCheck:= 2 else
            BlockDoubleCheck:= 0;
         end; {BlockDoubleCheck}
      Function NumOfChecks(Board:BoardType ; Move:Char):integer;
         {Returns one point per check given a particular move}
         begin
            NumOfChecks:= CheckOutSpot(board,move,false);
         end; {NumOfChecks}
      Function BestSpot(spots:SpotsType):char;
         {Takes the array an returns the char corresponding to the space with
          the most points}
         var GoodSpot:Record Points:integer; spot:char; end;
             a:char;
         begin
            GoodSpot.Points:=0; GoodSpot.Spot:='1';
            For a:= '1' to '9' do
               if (spots[a]>GoodSpot.Points) then
                  begin GoodSpot.Points:=spots[a]; GoodSpot.Spot:=a; end;
            BestSpot:= GoodSpot.Spot;
         end; {BestSpot}
      begin
         for a:= '1' to '9' do
            begin
            spots[a]:= AccountForValidity(board,a);
               if (spots[a]=1) and (Opponent=JJ) then
                  spots[a]:= spots[a] +
                  BlockDoubleCheck(board,a) +
                  NumOfChecks(board,a)
               else if (spots[a]=1) and (Opponent=Computer) then
                  spots[a]:= spots[a] + NumOfChecks(board,a)
            end; {for}
         board.ExecuteMove(BestSpot(spots),false);
      end; {CompuType.GetGoodSpot}

   Procedure CompuType.FirstMove(PlayersMove:char ; var Board:BoardType);
      {see notes at top of unit}
      begin
         Delay(ComputerDelayTime);
         case PlayersMove of
         {cm}     '7','9','1','3' : Board.ExecuteMove('5',false);
         {ssOpp}  '8'             : Board.ExecuteMove('2',false);
                  '4'             : Board.ExecuteMove('6',false);
                  '6'             : Board.ExecuteMove('4',false);
                  '2'             : Board.ExecuteMove('8',false);
         {mc}     '5'             : Case Random(3) of
                                       0 : Board.ExecuteMove('7',false);
                                       1 : Board.ExecuteMove('9',false);
                                       2 : Board.ExecuteMove('1',false);
                                       3 : Board.ExecuteMove('3',false);
                                    end; {inner case}
         end; {case}
      end; {CompuType.FirstMove}

   Procedure CompuType.SelectMove(var board:BoardType ;
      Opponent:OpponentType);
      begin
         Delay(ComputerDelayTime);
         if TryCheckMate(board) then {exit}
         else if BlockCheckMate(board) then {exit}
         else if BlockDiagnolSituation(board) then {exit}
         else GetGoodSpot(board,Opponent); {last alternative}
      end; {CompuType.SelectMove}

   Procedure HypotheticalBoard.HypExeMove(choice:char ; Oturn:boolean);
      {Enter it into the spaces array.}
      begin
         if OTurn then spaces[choice]:= O
            else spaces[choice]:= X;
      end; {HypotheticalBoard.HypExeMove}

   Function HypotheticalBoard.HypFinish : FinishType;
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
      Function CheckForCatsGame : Boolean;
         var Return:boolean;
             a:char;
         begin
            Return:= true;
            for a:= '1' to '9' do
               if spaces[a]=Nothing then Return:= false;
            CheckForCatsGame:= Return;
         end; {CheckForCatsGame}
      begin {BoardType.HypFinish}
         if Winner then
            HypFinish:= win
         else if CheckForCatsGame then HypFinish:= cat
         else HypFinish:= no;
      end; {BoardType.HypFinish}

begin
   randomize;
end.