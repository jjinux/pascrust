{t                              Shannon (JJ) Behrens
                                10/17/97

For information on BlackJack, please see procedure information.}

program BlackJack;

uses Crt;

type
   card  = (ha, h2, h3, h4, h5, h6, h7, h8, h9, h10, hj, hq, hk,
             da, d2, d3, d4, d5, d6, d7, d8, d9, d10, dj, dq, dk,
             sa, s2, s3, s4, s5, s6, s7, s8, s9, s10, sj, sq, sk,
             ca, c2, c3, c4, c5, c6, c7, c8, c9, c10, cj, cq, ck, nothing);
   {Nothing represents no real card in
    card's space.  Players will always
    have 11 cards.}
   spaces = array [0..10] of card;
   ending = (win, lose, tie);

var
   compcard  : spaces;                 {Computer's space for cards}
   playcard  : spaces;                 {Players space for cards}
   bet       : integer;
   wallet    : integer;
   playagain : char;
   Winner    : ending;
   seeninfo  : boolean;                {give ability to see info once}

function sumcards(cardset: spaces): integer;
   var
      tsum : integer;                  {will become value of sumcards}
      a, b : integer;
      high : boolean;                  {score over 21, A's become 1's}
      aces : integer;
   begin
      tsum := 0;
      aces := 0;
      for a := 0 to 10 do              {note:  aces are always treated like}
      begin
         case cardset[a] of            {11's at this point.  They will be}
            da, sa, ha, ca     : tsum := tsum + 11;   {changed to 1's later}
            d2, s2, h2, c2     : tsum := tsum + 2;
            d3, s3, h3, c3     : tsum := tsum + 3;
            d4, s4, h4, c4     : tsum := tsum + 4;
            d5, s5, h5, c5     : tsum := tsum + 5;
            d6, s6, h6, c6     : tsum := tsum + 6;
            d7, s7, h7, c7     : tsum := tsum + 7;
            d8, s8, h8, c8     : tsum := tsum + 8;
            d9, s9, h9, c9     : tsum := tsum + 9;
            d10, s10, h10, c10 : tsum := tsum + 10;
            dj, sj, hj, cj     : tsum := tsum + 10;
            dq, sq, hq, cq     : tsum := tsum + 10;
            dk, sk, hk, ck     : tsum := tsum + 10;
            nothing            : tsum := tsum;
         end; {case}
      end; {for loop}
      for b := 0 to 10 do                    {Adjusts for aces}
         case cardset[b] of
            da, sa, ha, ca: inc(aces);       {how many are there?}
         end; {case}
      while (tsum>21) and (aces>0) do        {is score too high}
      begin                                  {if Ace is present, take away}
         tsum := tsum - 10;                  {10 points until score is low}
         dec(aces);                          {enough, or there are no more}
      end;                                   {aces to change}
      sumcards := tsum;
   end; {sumcards}

procedure information;
   begin
      clrscr;
      writeln('t');
      writeln;
      writeln('Shannon''s Casino was programmed by Shannon Behrens, Merced California, 1997.');
      writeln;
      writeln('The dealer here follows normal dealer strategy, but unlike most casinos, there');
      writeln('is only one deck (i.e. there are no duplicate cards).  This was done in honor');
      writeln('of my grandmother who said it is more fun when you can count the cards!');
      writeln;
      writeln('Any questions should be mailed to');
      writeln('   Shannon Behrens');
      writeln('   16059 Sunburst St.');
      writeln('   North Hills, Ca. 91343');
      writeln;
      writeln('Thanks for playing.');
      repeat until keypressed;
      clrscr;
      writeln;
   end; {information}

procedure IntroGame (var wallet: integer; var bet: integer; playagain: char;
                     var seeninfo: boolean);
   {also sets up the screen colors}
   procedure WannaSeeInfo;
      {note, this will only ever be printed once}
      var currx, curry: integer;
      begin
         currx := wherex; curry := wherey;
         gotoxy(30, hi(windmax));
         write('For a free tour of the Shannon''s casino, bet -$1');
         gotoxy(currx, curry);
      end; {wannaSeeInfo}

   begin
      if playagain = 'n' then          {i.e. first time only}
      begin
         writeln; writeln; writeln;
         writeln('That extremely large lump in your pocket is');
         writeln('$', wallet, ' worth of pocker chips.');
      end;
      bet := -3700;                  {just a flag}
      repeat
         if bet = -1 then information
         else if (bet <> -3700) and (bet<0) then
            writeln('If that''s the way you feel, you ought to leave!')
         else if (bet <> -3700) and (bet>wallet) then
         begin
            writeln('Sorry, we don''t offer loans.  You only have ');
            writeln('$', wallet, ' in your wallet.');
         end; {else}
         write('How much would you like to bet?  ');
         if not seeninfo then WannaSeeInfo;
         seeninfo := true;
         readln(bet);
      until (bet>0) and (bet <= wallet);
      wallet := wallet-bet;          {notice that this is where the player
                                      loses his money}
   end; {introGame}

function getcards(playcard: spaces; compcard: spaces): card;
   var
      b: integer;
      tcard: card;                   {temp var for getcards -- will = it}
      repeats: boolean;              {only one deck, getcards will not}
   begin                             {give same card twice}
      repeat
         tcard := card(random(52)+ord(ha));
         repeats := false;
         for b := 0 to 10 do
            if (playcard[b] = tcard) or (compcard[b] = tcard)
               then repeats := true;
      until repeats = false;
      getcards := tcard;
   end;  {getCards}

procedure setup(var compcard: spaces; var playcard: spaces);
   var
      a: integer;                    {for loops}
   begin
      randomize;
      for a := 0 to 10 do
         compcard[a] := nothing;
      for a := 0 to 10 do
         playcard[a] := nothing;
      compcard[0] := getcards(playcard, compcard);
      {player only gets to see one of comp's
       cards at this point};
      for a := 0 to 1 do             {player gets to see 2 of his own}
         playcard[a] := getcards(playcard, compcard);
   end; {setup}

procedure showcards(compcard: spaces; playcard: spaces);
   var
      a, b: integer;
   procedure writecard(given: card);    {sub procedure:  translates 1 card}
      begin                             {to something printable}
         textcolor(green);
         case given of
            ha      : write('Ace ', chr(3), '   ');
            h2      : write('2 ', chr(3), '   ');
            h3      : write('3 ', chr(3), '   ');
            h4      : write('4 ', chr(3), '   ');
            h5      : write('5 ', chr(3), '   ');
            h6      : write('6 ', chr(3), '   ');
            h7      : write('7 ', chr(3), '   ');
            h8      : write('8 ', chr(3), '   ');
            h9      : write('9 ', chr(3), '   ');
            h10     : write('10 ', chr(3), '   ');
            hj      : write('Jack ', chr(3), '   ');
            hq      : write('Queen ', chr(3), '   ');
            hk      : write('King ', chr(3), '   ');
            da      : write('Ace ', chr(4), '   ');
            d2      : write('2 ', chr(4), '   ');
            d3      : write('3 ', chr(4), '   ');
            d4      : write('4 ', chr(4), '   ');
            d5      : write('5 ', chr(4), '   ');
            d6      : write('6 ', chr(4), '   ');
            d7      : write('7 ', chr(4), '   ');
            d8      : write('8 ', chr(4), '   ');
            d9      : write('9 ', chr(4), '   ');
            d10     : write('10 ', chr(4), '   ');
            dj      : write('Jack ', chr(4), '   ');
            dq      : write('Queen ', chr(4), '   ');
            dk      : write('King ', chr(4), '   ');
            sa      : write('Ace ', chr(6), '   ');
            s2      : write('2 ', chr(6), '   ');
            s3      : write('3 ', chr(6), '   ');
            s4      : write('4 ', chr(6), '   ');
            s5      : write('5 ', chr(6), '   ');
            s6      : write('6 ', chr(6), '   ');
            s7      : write('7 ', chr(6), '   ');
            s8      : write('8 ', chr(6), '   ');
            s9      : write('9 ', chr(6), '   ');
            s10     : write('10 ', chr(6), '   ');
            sj      : write('Jack ', chr(6), '   ');
            sq      : write('Queen ', chr(6), '   ');
            sk      : write('King ', chr(6), '   ');
            ca      : write('Ace ', chr(5), '   ');
            c2      : write('2 ', chr(5), '   ');
            c3      : write('3 ', chr(5), '   ');
            c4      : write('4 ', chr(5), '   ');
            c5      : write('5 ', chr(5), '   ');
            c6      : write('6 ', chr(5), '   ');
            c7      : write('7 ', chr(5), '   ');
            c8      : write('8 ', chr(5), '   ');
            c9      : write('9 ', chr(5), '   ');
            c10     : write('10 ', chr(5), '   ');
            cj      : write('Jack ', chr(5), '   ');
            cq      : write('Queen ', chr(5), '   ');
            ck      : write('King ', chr(5), '   ');
            nothing : write('');
         end; {case}
         textcolor(yellow);
      end; {writecard}
   begin {showcards}
      clrscr;
      writeln;
      writeln('                      Welcome to Shannon''s Blackjack Casino');
      gotoxy(1, 5);
      writeln('Computer''s cards:');
      writeln;
      write('     ');
      for a := 0 to 10 do            {space by space (11 in all)}
         writecard(compcard[a]);     {write to screen contents of space}
      writeln; writeln;              { - basically, translate from card to}
      writeln('Your cards:');        {something printable}
      writeln;
      write('     ');
      for a := 0 to 10 do
         writecard(playcard[a]);
      writeln; writeln;
   end; {showcards}

procedure moreCards(var playcard: spaces; compcard: spaces; counter: integer);
   var
      a: integer;
   begin
      if counter<10 then
      begin
         a := 0;                         {find first empty card and put a card}
         while playcard[a] <> nothing do a := a+1;   {in it}
         playcard[a] := getcards(playcard, compcard);
         counter := counter+1;
      end
      else writeln('I''m sorry you''re taking too many cards');
   end; {moreCards}

procedure dealerCards(var compcard: spaces; playcard: spaces);
   var
      a: integer;
   begin;
      a := 1;                      {at this point dealer already has a card}
      repeat                            {standard dealer strategy}
         compcard[a] := getcards(playcard, compcard);
         a := a + 1;
         showcards(compcard, playcard);
         delay(1000);
      until sumcards(compcard) >= 17;
   end; {dealerCards}

function calculate(playcard: spaces; compcard: spaces): ending;
   {decides whether player: win lose tie}
   var    player, computer: integer;  {to hold sum of cards}
   begin
      player := sumcards(playcard);
      computer := sumcards(compcard);
      if player = computer then
         calculate := tie
      else if (player>21) and (computer>21) then
         calculate := tie
      else if (player>21) then
         calculate := lose
      else if (computer>21) then
         calculate := win
      else if computer>player then
         calculate := lose
      else if computer<player then
         calculate := win;
   end; {calculate}

procedure fundage (wallet: integer);
   {prints to screen contents of wallet}
   var currx, curry: integer;
   begin
      currx := wherex;                {to return cursor to orig. pos}
      curry := wherey;
      gotoxy(62, 5);
      write('Fundage:  ');
      textcolor(green);
      writeln('$', wallet);
      textcolor(yellow);
      gotoxy(currx, curry);
   end; {fundage}

procedure prizes(bet: integer; var wallet: integer);
   begin;
      writeln('Congratulations, you won $', 2*bet, '!');
      wallet := wallet + 2*bet;
      delay(1000);
      fundage(wallet);
   end; {prizes}

procedure loses(bet: integer; var wallet: integer);
   begin
      writeln('I''m sorry, you lost,');
      if wallet>0 then
         writeln('but you still have $', wallet, ',')
      else writeln('tough luck kid!');
      delay(1000);
      fundage(wallet);
   end; {loses}

procedure wantmore(var playcard: spaces; compcard: spaces);
   var
      counter: integer;                 {insures player won't take too many cards}
      anothercard: char;                {answer to ? - do you want another?}
   begin                                {place where player gets more cards}
      counter := 0;
      repeat
         writeln('Would you like another card?');
         write('(y or n)  ');
         anothercard := readkey;
         write(anothercard);
         if upcase (anothercard) = 'Y' then
         begin
            moreCards(playcard, compcard, counter); {get another card}
            showcards(compcard, playcard);          {show new setup}
         end; {of if statement}
      until (upcase(anothercard) <> 'Y') or (counter = 10);
   end; {wantmore}

procedure Goodbye;
   var a: integer;
       forward, down: boolean;
      begin
      writeln;
      writeln('You currently have $', wallet, ' in your wallet.');
      textcolor(green);
      writeln;
      readln;
      clrscr;
      forward := true;
      down := true;
      repeat                          {writeln over and over, bouncing it}
         if forward then              {around on the screen}
            gotoxy(wherex+1, wherey)
         else gotoxy(wherex-1, wherey);
         if down then
            gotoxy(wherex, wherey+1)
         else gotoxy(wherex, wherey-1);
         if wherex<2 then
            forward := true
         else if wherex>38 then
            forward := false;
         if wherey<2 then
            down := true
         else if wherey>hi(windmax) then
            down := false;
         if down then textcolor(yellow)
            else textcolor(green);
         write('Thanks for playing at Shannon''s Casino!!!');
         delay((hi(windmax)-wherey)*20+20);    {simulates falling, not special}
         gotoxy(wherex-41, wherey);         {numbers}
      until keypressed;
      textcolor(yellow);
      readln;
      end; {goodbye}

      procedure PTie(bet: integer; var wallet: integer);
      begin
      writeln('It''s a Draw!');
      writeln;
      writeln('You have $', wallet, ' in your wallet, ');
      writeln('   and $', bet, ' ready to be bet again.');
      wallet := wallet + bet;
      delay(1000);
      fundage(wallet);
      end; {tie}

      procedure Welcome(var playagain: char; var wallet: integer; var seeninfo:
                boolean);
      {several things are initialized here, including the screen and seeninfo}
      begin
      textcolor(green+blink);
      textbackground(blue);
      seeninfo := false;
      clrscr;
      playagain := 'n';
      writeln;
      writeln('                     WELCOME TO SHANNON''S BLACKJACK CASINO!');
      writeln;
      wallet := 20;
      textcolor(yellow);
      end; {welcome}

      procedure recooperate(var wallet: integer; var bet: integer;
                    var playagain: char);
      {A slight twist on the normal ending of a game, this is used to reflect
       the severity of losing all your money}
      begin
      clrscr;
      textcolor(yellow+blink);
      gotoxy(wherex, 7);
      writeln('               /--    _         __   /--\ |  |  __  /-\   | ');
      writeln('              | ___  /_\ | \/| |__  |   | |  | |__  | /   | ');
      writeln('               \-|   | | |   | |__   \__/  \/  |__  |  \  * ');
      textcolor(yellow);
      gotoxy(1, hi(windmax)-2);
      writeln('Would you like to play again?');
      write('(y or n):  ');
      playagain := upcase(readkey);
      if playagain = 'Y' then
         Welcome(playagain, wallet, seeninfo);  {start at beginning}
      end;

      begin
      Welcome(playagain, wallet, seeninfo);
      repeat
      IntroGame (wallet, bet, playagain, seeninfo);
      setup(compcard, playcard);
      showcards(compcard, playcard);
      wantmore(playcard, compcard);
      dealerCards(compcard, playcard);
      Winner := calculate(playcard, compcard);
      if Winner = win then prizes(bet, wallet)
         else if Winner = lose then loses(bet, wallet)
         else if Winner = tie then PTie(bet, wallet);
      if wallet>0 then
         begin
         write('Would you like to play again (y or n)?  ');
         delay(500);
         playagain := upcase(readkey);
         writeln;
         end;
      if wallet = 0 then recooperate(wallet, bet, playagain);
      until playagain = 'N';
      Goodbye;

      end.                              {fix getcards}
