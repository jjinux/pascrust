{t
                                Shannon Behrens
                                Computer Programming II
                                2/18/97

The Game of Life - this is a computer game in which a 2*D grid is set up
    using arrays.  Each space is a boolean.  If the boolean is true, the
    grid contains a "bug."  Depending on each space's neighbors, a bug
    may be born, may die, may continue to live, or may continue to not
    exist.  Each "Generation" the computer will cycle through, decide
    decide these things for each space, and print the grid and number
    of generations.}

program Life;

uses Crt;

const
   maxrows = 21;
   maxcol  = 77;                     {rows & columns in grid}
   starting = 7;                     {1..inf, the higher number the fewer
                                      the bugs at start}

type                                 {2*D grid made to fit screen}
   gridT = array[0..maxcol, 0..maxrows] of boolean;

var
   generations : integer;            {# of generations shown}
   grid        : gridT;              {Grid where 'bugs' live}
   leave       : boolean;            {players signal to leave game}
   choicerows  : integer;            {player's choice of board size}
   choicecols  : integer;
   mutated     : boolean;            {see options
    - allows random bug type}

procedure setup(var grid: gridT; var generations: integer;
                var choicerows, choicecols: integer; var mutated: boolean);
   var
      r, c: integer;                 {r=row c=column}
   begin
      mutated := false;
      textbackground(blue);
      textcolor(yellow);
      generations := 0;
      choicerows := maxrows;         {begin with maximum board size}
      choicecols := maxcol;          {and random bug generations}
      for r := 0 to choicerows do
         for c := 0 to choicecols do
            grid[c, r] := false;        {no bugs yet}
      for r := 1 to (choicerows-1) do   {don't want bugs on edges}
         for c := 1 to (choicecols-1) do
            if random(starting) = 1 then     {chance a bug is in spot at start}
               grid[c, r] := true
            else grid[c, r] := false;
   end; {setup}

procedure information; {text}
   begin
      clrscr;
      writeln('t                                  Life');
      writeln;
      writeln('Aye, la vida!!! No?  Life is an artificial life simulation involving');
      writeln('bugs on a two dimensional grid.  Each bug has eight neighbors.  If less');
      writeln('than two of his neighbors are alive, he dies of loneliness.  If more');
      writeln('than three of his neighbors are alive, he dies of overcrowding.  If an');
      writeln('empty space has exactly three neighbors, a baby bug is born into that');
      writeln('space.');
      writeln;
      writeln('Life has many options in which you can change the size of the grid,');
      writeln('plant your own bugs, or skip to a certain generation in the future.');
      writeln('Every time the computer decides who lives and who dies, one generation');
      writeln('passes.');
      writeln;
      writeln('                       Life was programmed by Shannon Behrens, 2/25/97');
      repeat until keypressed;
   end; {information}

procedure Changesize(var grid: gridT; var choicerows, choicecols: integer);
   {shows generic grid, allows user to use 8462 (arrow keys)}
   var                   {to change size.  Bugs outside of grid are deleted.}
      choice: char;      {Necessary to exit changesize loop}

   function GetChoice: char;
      begin              {gets a 8,4,6,2, or 'X' so that user can change board size}
         gotoxy(1, 7);       {board size and then leave}
         writeln('           To change board size, make');
         writeln('           sure NumLock is ON, and use:');
         writeln;
         writeln('                        8');
         writeln('                      4   6');
         writeln('                        2');
         writeln;
         writeln('           Hit ''X'' when finished.');
         GetChoice := readkey;
      end; {getchoice}

   procedure killbugs(var grid: gridT; choicerows, choicecols: integer);
      var                                {if bug not in grid, kill it}
         r, c: integer;                  {r=row c=col}
      begin
         for r := 0 to choicerows do     {kill left unused portion of grid}
            for c := choicecols to maxcol do
               grid[c, r] := false;
         for r := (choicerows-1) to maxrows do
            for c := 1 to maxcol do      {kill bottom unused portion of grid}
               grid[c, r] := false;
      end; {killbugs}

   procedure showbox(grid: gridT; choicerows, choicecols: integer);
      var              {shows a blue box in order to show size of grid}
         r, c: integer;                  {r=row c=column}
      begin
         textbackground(blue);
         for r := 0 to choicerows do
         begin
            for c := 0 to choicecols do
               write(' ');
            writeln;                     {start new row after each for}
         end; {for loop}
      end; {showbox}

   begin {changesize}
      repeat
         textbackground(black);
         clrscr;
         showbox(grid, choicerows, choicecols);
         choice := GetChoice;         {will use choice to exit loop}
         case upcase(choice) of       {care must be taken to have limits on grid size}
            '8': if choicerows>2 then
               choicerows := choicerows-1;
            '4': if choicecols>2 then
               choicecols := choicecols-1;
            '6': if choicecols<maxcol then
               choicecols := choicecols+1;
            '2': if choicerows<maxrows then
               choicerows := choicerows+1;
            'X', 'E': writeln('t');
            {else do nothing, we're repeating}
         end; {case}
      until (upcase(choice) = 'X') or (upcase(choice) = 'E');
      killbugs(grid, choicerows, choicecols);
   end; {changesize}

procedure Changebugs(var grid: gridT; choicerows, choicecols: integer);

   var
      choice: char;          {Players choice to move, place bug, or leave.}
      currx, curry: integer; {Stores position so that player can move around}
   {screen and place bugs.}
   procedure instructions;
      begin
         clrscr;
         gotoxy(1, 7);
         writeln('           To place bugs, make sure');
         writeln('           NumLock is ON, and use:');
         writeln;
         writeln('                        8');
         writeln('                      4   6');
         writeln('                        2');
         writeln;
         writeln('           Use the spacebar to place or delete a bug.');
         writeln('           (type anykey to exit)');
         repeat until keypressed;
      end; {Changebugs}

   procedure DeleteAll(var grid: gridT; choicerows, choicecols: integer);
      var
         r, c: integer;               {r=row c=column}
      begin
         for r := 0 to choicerows do
            for c := 0 to choicecols do
               grid[c, r] := false;      {deletes them all}
      end; {DeleteAll}

   procedure RandomBugs(var grid: gridT; choicerows, choicecols: integer);
      var
         r, c: integer;                 {r=row c=column}
         chances: integer;              {chances a bug will be in spot}
      begin                             {1-25 with 1 being 1/2 bugs}
         DeleteAll(grid, choicerows, choicecols);
         repeat
            clrscr;
            gotoxy(7, 7);
            writeln('                    How many bugs do you want?');
            writeln('                      (1 - a lot   to   50 - very few)');
            gotoxy(28, 10);
            readln(chances);
         until (chances >= 0) and (chances<51);
         chances := chances + 1;        {else a 1 leads to no bugs}
         for r := 1 to (choicerows-1) do {don't want bugs on edges}
            for c := 1 to (choicecols-1) do
               if random(chances) = 1 then
                  grid[c, r] := true
               else grid[c, r] := false;
      end; {RandomBugs}

   procedure LikeShowScreen(grid: gridT; choicerows, choicecols: integer);
      var
         r, c: integer;               {r=row c=col}
      begin                           {Like it, but bottom is different}
         clrscr;
         for r := 0 to choicerows do
         begin
            for c := 0 to choicecols do
               if grid[c, r] then write('*') {if alive, draw bug}
               else write(' ');              {otherwise, leave a space}
            writeln;                         {start new row}
         end; {for loop}
         writeln('Spacebar - delete or add a bug,');
         writeln('''X'' - exit        ''H'' - help.');
         writeln('''A'' - delete all  ''R'' - random');
      end; {LikeShowScreen}

   begin {Changebugs}
      clrscr;
      currx := 7;
      curry := 7;
      LikeShowScreen(grid, choicerows, choicecols);
      repeat
         gotoxy(currx, curry);
         choice := readkey;
         case upcase(choice) of
            '8' : if curry>1           then curry := curry-1;
            '4' : if currx>2           then currx := currx-1;
            '6' : if currx<choicecols  then currx := currx+1;
            '2' : if curry<choicerows-1
                                            then curry := curry+1;
            ' ': begin                  {This is here so that you don't have to keep showscreening}
               grid[currx-1, curry] := not grid[currx-1, curry]; {the write ('*') and (' ')}
               if grid[currx-1, curry] then write('*')           {have no affect on grid,}
               else write(' ');                                  {they're for viewing only.}
            end;
            'H' : instructions;
            'A' : DeleteAll(grid, choicerows, choicecols);
            'R' : RandomBugs(grid, choicerows, choicecols);
            else writeln('');       {Do nothing, it's looping anyway}
         end; {choice}
         if (choice <> '8') and (choice <> '4') and (choice <> '6') and (choice <> '2')
            and (choice <> ' ')               {rebring menu if necessary}
            then LikeShowScreen(grid, choicerows, choicecols);
      until (upcase(choice) = 'E') or (upcase(choice) = 'X');
   end; {changebugs}

procedure Mutation(var mutated: boolean);
   var
      choice: char;                   {y or n to mutation}
   begin
      clrscr;
      gotoxy(1, 7);
      writeln('                       Welcome to the mutations menu:');
      writeln;
      writeln('                           ooo         ^ &   -                ');
      writeln('                          o * o ---> ! @ # $   %              ');
      writeln('                           ooo           @                    ');
      writeln('                                        (*)<---- Um!  Yummy!');
      writeln;
      repeat
         write('                 Would you like to be mutated? (y or no)  ');
         choice := readkey;
         writeln;
         if (upcase(choice) <> 'Y') and (upcase(choice) <> 'N') then
            writeln('                   I''m sorry, try again.');
      until (upcase(choice) = 'Y') or (upcase(choice) = 'N');
      writeln;
      if upcase(choice) = 'Y' then
      begin
         mutated := true;
         textcolor(yellow+blink);
         writeln('                                   You''re mutated!!!');
         textcolor(yellow);
      end  {before else}
      else
      begin
         mutated := false;
         writeln('                                 Square!!!');
      end; {else}
      repeat until keypressed;
   end; {mutation}

procedure Options(var grid: gridT; var choicerows, choicecols: integer;
                  generations: integer; var mutated: boolean);
   {menu may lead to weirdobugs}
   var
      choice: char;                    {users option}
   begin
      repeat
         clrscr;
         gotoxy(1, 7);
         writeln('       Welcome to Options, please choose:');
         writeln;
         writeln('                1.  Board Size');                 {Changesize}
         writeln('                2.  Add or Delete Organisms');    {Changebugs}
         writeln('                3.  Mutate/Unmutate Organisms');  {random bug type}
         writeln('                4.  Facts of Life');              {information}
         writeln('                5.  Exit');
         choice := readkey;
         case upcase(choice) of
            'B', 'S', '1' : Changesize(grid, choicerows, choicecols);
            'A', 'D', '2' : Changebugs(grid, choicerows, choicecols);
            'M', 'O', '3' : Mutation(mutated);
            'F', 'L', '4' : information;
            {else do nothing, we're repeating anyway}
         end; {case}
      until (upcase(choice) = '5') or (upcase(choice) = 'X');
   end; {options}

procedure WelcomeMenu(var grid: gridT; var choicerows, choicecols: integer;
                      var mutated: boolean);
   {var parameters because WMenu can lead to options}
   var
      choice: char;                  {users option}

   begin
      repeat
         clrscr;
         gotoxy(1, 7);
         writeln('                Welcome, please choose:');
         writeln;
         writeln('                1.  Play');
         writeln('                2.  Change Settings');      {goto options}
         writeln('                3.  Facts of Life');        {goto information}
         choice := readkey;
         case upcase(choice) of
            'P', '1'      : writeln('');
            'C', 'S', '2' : Options(grid, choicerows, choicecols,
                                      generations, mutated);
            'F', 'L', '3': information;
            {else do nothing, we're repeating anyway}
         end; {case}
      until (upcase(choice) = 'P') or (upcase(choice) = '1');
   end; {WelcomeMenu}

procedure regenerate(var grid: gridT; var generations: integer;
                     choicerows, choicecols: integer);
   var                          {Decides who lives.  Increments generations}
      r, c      : integer;      {r=row c=column}
      neighbors : integer;      {amt. of live bugs surrounding space/bug}
      tempgrid  : gridT;        {wait and see who's dead/alive then change board}
   begin
      for r := 0 to choicerows do
         for c := 0 to choicecols do
            tempgrid[c, r] := false;            {no bugs yet}
      for r := 1 to choicerows-1 do
         for c := 1 to (choicecols-1) do        {sideline bugs are always dead}
         begin
            neighbors := 0;                     {count neighbors}

            if grid[c-1, r-1]         then neighbors := neighbors+1;
            if grid[c, r-1]           then neighbors := neighbors+1;
            if grid[c+1, r-1]         then neighbors := neighbors+1;
            if grid[c-1, r]           then neighbors := neighbors+1;
            if grid[c+1, r]           then neighbors := neighbors+1;
            if grid[c-1, r+1]         then neighbors := neighbors+1;
            if grid[c, r+1]           then neighbors := neighbors+1;
            if grid[c+1, r+1]         then neighbors := neighbors+1;

            if grid[c, r] then     {now, is space alive or dead?}
            begin
               if (neighbors<2) or (neighbors>3) then tempgrid[c, r] := false
               else tempgrid[c, r] := true
            end
            else if neighbors = 3
               then tempgrid[c, r] := true;
         end; {for loop}
      grid := tempgrid;
      generations := generations+1;
   end; {regenerate}

procedure Pause(var grid: gridT; var choicerows, choicecols: integer;
           var leave: boolean; var generations: integer; var mutated: boolean);
   {var parameters because may lead to options}
   var
      choice: char;                 {user's choice}

   procedure showmenu;
      begin
         clrscr;
         gotoxy(25, 7);             {now, fill in letters}
         writeln;
         writeln('          Pause Menu - Please choose:        ');
         writeln('            1.  Exit                        ');
         writeln('            2.  Jump Ahead X Generations    ');
         writeln('            3.  Options                     ');
         writeln('            4.  StartOver                   ');
         writeln;
         writeln('            5.  Die...                      ');
         writeln;
      end; {showmenu}

   procedure JumpAhead(var grid: gridT; var generations: integer;
                       choicerows, choicecols: integer);
      var           {player chooses how many generations to skip}
         GenPlus : integer; {how many generations to skip}
         a       : integer;
      begin
         clrscr;
         gotoxy(7, 7);
         write('How many generations would you like to skip?');
         readln(GenPlus);
         for a := 1 to GenPlus do
         begin
            regenerate(grid, generations, choicerows, choicecols);
            if (a mod 20) = 1 then
            begin
               clrscr;
               gotoxy(25, 7);
               writeln('Calculations ', a/GenPlus*100:0:0, '% complete.');
            end;
         end;
      end; {JumpAhead}

   procedure StartOver(var grid: gridT; var generations: integer;
                       choicerows, choicecols: integer);
      begin
         setup(grid, generations, choicerows, choicecols, mutated);
         WelcomeMenu(grid, choicerows, choicecols, mutated);
         {menu may access options or information}
      end; {StartOver}

   begin {pause}
      repeat
         showmenu;
         choice := readkey;
         case upcase(choice) of
            '1', 'R', 'L' :  {Exit};
            '2', 'J', 'G' : JumpAhead(grid, generations, choicerows,
                                       choicecols);
            '3', 'C', 'S' : Options(grid, choicerows, choicecols, generations, mutated);
            '4', 'S', 'O' : StartOver(grid, generations, choicerows,
                                       choicecols);
            '5', 'D': begin
               leave := true;
               choice := 'X';
            end;
         end; {case}
      until (upcase(choice) = 'X') or (upcase(choice) = 'R') or (choice = '1')
         or (upcase(choice) = 'S') or (choice = '4') or (choice = 'O');
   end; {pause}

procedure showscreen(grid: gridT; generations: integer;
                     choicerows, choicecols: integer; mutated: boolean);
   var            {draws bug grid, and displays generations}
      r, c: integer;                 {r=row c=column}

   procedure getbug(mutated: boolean);
      {draws one bug}
      var
         Rand: integer;               {holds value of random number}
      begin
         Rand := random(11);
         if not mutated then write('*')
         else case Rand of
            0 : write (')');
            1 : write ('!');
            2 : write ('@');
            3 : write ('#');
            4 : write ('$');
            5 : write ('%');
            6 : write ('^');
            7 : write ('&');
            8 : write ('*');
            9 : write (')');
         end; {case}
      end; {getbugs}

   begin {showscreen}
      clrscr;
      for r := 0 to choicerows do
      begin
         for c := 0 to choicecols do
            if grid[c, r] then getbug(mutated) {if alive, draw bug}
            else write(' ');                   {otherwise, leave a space}
         writeln;                              {start new row}
      end; {for loop}
      writeln('Number of Generations:  ', generations);
   end; {showscreen}

procedure Goodbye(generations: integer);
   begin
      writeln;
      writeln('You witnessed ', generations-1, ' generations.');
      writeln('Goodbye');
      readln;
   end; {Goodbye}

begin
   randomize;
   setup(grid, generations, choicerows, choicecols, mutated);
   WelcomeMenu(grid, choicerows, choicecols, mutated);
   leave := false;                                          {menu may access options or information}

   repeat                                                   {cycle of life}
      showscreen(grid, generations, choicerows, choicecols, {shows bugs & generations}
                 mutated);
      regenerate(grid, generations, choicerows, choicecols); {decides who will live and generations++}
      writeln('Hit any key to continue, or ''p'' to pause');
      if upcase(readkey) = 'P'                              {-->pause menu, options menu}
         then Pause(grid, choicerows, choicecols, leave, generations, mutated);
   until leave;                                             {or exit}

   Goodbye(generations);
end.
