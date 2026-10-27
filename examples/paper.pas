{Shannon Behrens
Computer Programing I
12/4/96

Homework for chapter 10: T2.
This program is the age old game of paper, scissors, rocks.  It is intended
for one player to play against the computer.}

Program ChineseGame;

uses Crt;

Type
   choices = (paper, scissors, rock);
   outcome = (win, lose, tie);
   outta5  = 0..5;

Var
   name     : string;                       {request player by name}
   moveComp : choices;                      {player's choice}
   movePlay : choices;                      {computer's choice}
   wins     : outta5;                       {computer wins with 5 wins}
   losses   : outta5;                       {player wins if comp loses 5 times}
   i        : integer;

Procedure IntroSetup(var wins, losses : outta5; var name : string);
{Initializes program for play, introduces and intices player}
   begin;
      randomize;
      wins:= 0;
      losses:= 0;
      textcolor(yellow);
      textbackground(blue);
      writeln('t');
      for i:= 1 to 55 do writeln;
      writeln('Paper, Scissors, Rocks, you know the game.');
      writeln('Now prepare to lose to Geobuddy''s evil twin brother!!!');
      writeln('Ha, Ha, Ha!!!', chr(10));
      writeln('What do you call yourself, human?');
      readln(name);
      writeln;
      writeln('Well, anyway, here''s the deal sparky, no funny stuff!');
      writeln('The first one to win 5 times sleeps soundly.  The other');
      writeln('has to live with the knowledge that he sucks!!!');
      writeln('Let''s play, ', name, chr(10), chr(10), chr(10));
      readln;
   end; {InroSetup}

function Readmove : choices;
{gets a relatively safe response for player's choice of moves}
   var
      choice : char;
      tempf  : choices;             {pre-end function value}
      okay   : boolean;             {checks for bad imput}
   begin
      repeat
      okay:= true;
      writeln('What''ll you choose -- Paper, Rock, or Scissors?');
      readln(choice);
      writeln;
      case upcase(choice) of
         'P'     :   tempf:= paper;
         'S'     :   tempf:= scissors;
         'R'     :   tempf:= rock;
         else okay:= false;         {bad input}
      end; {case}
      if not okay then              {scolds user, re-enter input}
         writeln('Quit your snivelling and speak up!!!');
      until okay;
      Readmove:= tempf;
   end;    {function Readmove}

procedure acceptMove(var MoveComp : choices);
{procedure accepts player's move}
var
   i : integer;
begin
      for i:= 1 to 33 do writeln;
      writeln(chr(10),'Make your move Misses ', name);
      MoveComp:= ReadMove;
end;   {procedure acceptMove}

Function MovePlayFunction(MovePlay : choices) : string;
{tells user of computer's choice}
var
   literature : string;                 {translates choices}
   str        : string;                 {buffer for function value}
begin
   case MovePlay of
      paper   :   literature:= 'Paper';
      scissors:   literature:= 'Scissors';
      rock    :   literature:= 'Rock';
   end;  {case}
   str:= ('I choose ' + literature + '.');
   MovePlayFunction:= str;
end;  {function MovePlayFunction}

procedure ChooseMove(var MovePlay : choices; moveComp : choices);
{Computer either cheats, using moveComp, or randomly selects a move}
   var
      cheater : integer;                  {1 out of 3 times computer will cheat}
   begin
      cheater:= random(2);                {if computer picks 2, it will cheat}
      if cheater <> 1 then
         MovePlay:= choices(random(3))
      else
         begin
         if moveComp = paper then
            MovePlay:= scissors;
         if moveComp = scissors then
            MovePlay:= rock;
         if moveComp = rock then
            MovePlay:= paper;
         writeln('Ha, Ha, Ha, you stink!!!');
         end; {of else}
   writeln(MovePlayFunction(MovePlay));
   end; {procedure ChooseMove}

procedure selectWinner (MoveComp, MovePlay : choices; var wins, losses : outta5);
{Take's player's move, compares it to computer's, and announces winner.  Then
adds to running total of score (wins, losses) }
var
   IWon : outcome;

   procedure AddUpTotals(var wins, losses : outta5; Iwon : outcome);
      {This sub procedure adds to running total of score (wins, losses), and throws
      in a few comments about winning or losing}
   begin
      writeln;
      if Iwon = win then
         begin
            wins:= wins + 1;
            writeln('Ha, Ha, Ha, I win, as usual.');
         end;
      if Iwon = lose then
         begin
            losses:= losses + 1;
            writeln('You won, grumble, grumble, grumble.');
         end;
      if Iwon = tie then
         writeln('I hate ties!!!  They imply equality.');
      readln;
   end;  {sub-procedure AddUpTotals}

begin
   if moveComp=paper then
      begin
         if moveplay = paper then Iwon := tie
         else if moveplay = scissors then Iwon := win
         else Iwon := lose;
      end
   else if moveComp=scissors then
      begin
         if moveplay = paper then Iwon := lose
         else if moveplay = scissors then Iwon := tie
         else Iwon := win;
      end
   else                                   {moveComp must equal rock}
      begin
         if moveplay = paper then Iwon := win
         else if moveplay = scissors then Iwon := lose
         else Iwon := tie;
      end;                                {of total if branching}
   AddUpTotals(wins, losses, Iwon);
end; {procedure selectWinner}

Procedure Congratulate(wins, losses : outta5);
 {declares winner and congratulates him}
var
   i : integer;
begin
   if wins>losses then
     begin
        textcolor(yellow + blink);
        writeln('Yeah, I won!!!  I love a fair game of paper, scissors, rocks!');
        textcolor(yellow);
     end {I win section}
   else
     begin
        textbackground(black);
        for i:= 1 to 33 do writeln;
        textcolor(red + blink);
        writeln('No...no...No!!!');
        writeln('I can''t stand it!', chr(7),chr(7),chr(7));
        writeln('Th..is can''t be happppening!!!  I''m dying!!');
        writeln('but you have''nt seen the last of me...');
        writeln('I''ll get you my pretty!!!!!');
        textcolor(yellow);
     end; {I lose section}
     readln;
end; {procedure Congratulate}

begin
   IntroSetup (wins, losses, name);
       if name='Chuckster' then                     {extra thrown in for testing}
          congratulate(0,5)
       else
       begin
   while (wins<5) and (losses<5) do
      begin
         acceptMove(MoveComp);
         chooseMove(MovePlay, MoveComp);
         selectWinner(MoveComp, MovePlay, Wins, losses);
      end;
   Congratulate(wins,losses);
       end;                                          {extra thrown in}
end.