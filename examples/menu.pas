{t                              Shannon Behrens
                                4/17/97

This program is a main menu for the computer.  At this point, changes to
the menu must be made through TP.

Any additions to this menu must be reflected in procedure Showchoices, const.,
and ExeChoice.}

program Menu;

uses Crt, Dos;
const
   SizeOfMenu = 'C';                    {Letter: last choice on menu}
   ScreenSaveTime = 10000;              {milisec's before ScrnSv}
type
   letter = 'A'..'Z';
   ChoicesType = array[letter] of string;
var
   choices   : ChoicesType;             {strings consisting of the choices}
   a         : letter;                  {index}
   selection : char;                    {choice selected}
   highlight : letter;                  {item cursor is on}
   leave     : boolean;                 {only way to leave the menu}
   pauser    : integer;                 {clock for screenSaver}

procedure setupwindow;
   begin
      textbackground(blue);
      textcolor(yellow);
   end; {SetupWindow}

procedure ShannonGames;
   {allows the user to see a list of my games, and then choose one to play.}
   var   choice: string;
      char1: char;                {first character of choice}

   procedure ListAndQuestion;
      {This subProc of ShannonGames.  It prints the list of games and
       promts the user to choose one to play}
      begin
         clrscr;
         setupwindow;
         writeln('                     This is a list of my games:');
         writeln;
         textcolor(green);
         writeln('  Life     ', '             2Cool');
         writeln('  Equatn   ', '             Airport4');
         writeln('  Sticks   ', '             BlkJack');
         writeln('  GmsCode  ', '             Paper (Rocks and Scissors)');
         gotoxy(1, hi(windmax)-2);
         textcolor(yellow);
         writeln('Please enter the name of the game');
         writeln('that you would like to play:  ');
         write('(Type X to exit)  ');
      end; {ListAndQuestion}

   begin {ShannonGames}
      repeat
         ListAndQuestion;
         textcolor(green);          {looks better}
         char1 := upcase(readkey);
         if char1 <> 'X' then
         begin                   {This is done in order to exit quickly}
            write(char1);        {via readkey, but still have a readln.}
            readln(choice);
            choice := char1 + choice;
            exec('a:\shannon\' + choice + '.exe', '');
            textcolor(blue);
            textbackground(white);
            if DosError <> 0 then
            begin
               gotoxy(45, 22);
               if DosError = 2 then
                  write(chr(7), 'I''m sorry, that ain''t one of ''em.')
               else write('Dos Error Number ', DosError);
               delay(2000);
            end; {if}
            textcolor(green);
            textbackground(blue);         {restore colors}
         end; {if}
      until char1 = 'X';
      setupwindow;                        {restore defaults}
   end; {ShannonGames}

procedure Exit(var leave: boolean);
   begin
      leave := true;
      textbackground(black);
      textcolor(white);
      clrscr;
      writeln('Thank you for using Shannon''s Menu');
   end; {Exit}

procedure ExeChoice(selection: char; var highlight: letter;
                    var leave: boolean);
   begin
      case selection of
         'A': begin
            exec('c:\tp\bin\turbo.exe', '');
            if DosError <> 0 then
            begin
               gotoxy(1, wherey+1);
               writeln(chr(7), 'Dos Error ', DosError);
            end; {if}
         end; {'A'}
         'B'     : ShannonGames;
         'C'     : Exit(leave);                {reset dos defaults}
         chr(13) : begin                       {translate.  65 is upcase A}
            selection := chr(ord(highlight));
            ExeChoice(selection, highlight, leave);
         end; {chr(13)}              {Exe. what highlight is on.}
         '8': if highlight <> 'A' then
            dec(highlight)           {wrapparound}
         else highlight := SizeOfMenu;
         '2': if highlight <> SizeOfMenu then
            inc(highlight)
         else highlight := 'A';
      end; {case}
   end; {ExeChoice}

procedure ScreenSaver;
   {A ScreenSaver for menu}
   var a, b: integer;
   begin
      textbackground(blue);
      clrscr;
      repeat
         textbackground(random(7));
         textcolor(random(7));
         a := random(9)+1;
         gotoxy(random(40), random(hi(windmax)));
         case a of
            1    : write('   Hello, please press some buttons!!!   ');
            2    : write('   I love You my Gina-Marie Ardizzone!!!   ');
            3    : write('   Help!!! I dropped my bag of chips!!!   ');
            4    : write('   Press ANYTHING to exit!!!   ');
            5    : write('   Shannon sleeps with a nightlight!!!   ');
            6..9 : begin
               textbackground(blue);
               gotoxy(1, random(hi(windmax)));
               for b := 1 to (lo(windmax)+1) do
                  write(' ');       {clear up the screen a bit}
            end; {6..9}
         end; {case}
         textcolor(blue);               {invisible cursor}
         delay(500);
      until keypressed;
      setupwindow;                      {restore window}
   end; {ScreenSaver}

procedure SetupThings(var leave: boolean; var choices: ChoicesType;
                      var highlight: letter);
   var a: letter;
   begin
      {$M $4000, 0, 0}                        {16K stack, no heap}
      randomize;
      leave := false;
      for a := 'A' to 'Z' do            {emptying array of choices}
         choices[a] := '';
      highlight := 'A';
   end; {SetupThings}

procedure showmenu(highlight: letter);
   {This procedure is very simple.  It contains two sub-procedures that
    it calls in order to do all the work}
   procedure ShowTitle;
      {This subProc of ShowMenu that prints out everything on the menu
       surrounding then choices}
      begin
         gotoxy(lo(windmin), hi(windmin));
         writeln;
         writeln('========================== Welcome To Shannon''s Menu ===========================');
         gotoxy(1, hi(windmax));
         writeln('Make sure NumLock is on.                                             Choice:___');
      end; {ShowTitle}

   procedure ShowChoices(highlight: letter);
      {Shows the user his potential choices.  Highlights item whose letter
       is equal to highlight.  This is a sub-Proc of MenuP}
      var  a: letter;
      begin
         textcolor(green);
         gotoxy(1, 5);                            {Store choices into array}
         choices['A'] := 'A. Turbo Pascal';       {Notice the format ?. ?????}
         choices['B'] := 'B. Shannon''s Games';
         choices['C'] := 'C. Exit to Dos';
         for a := 'A' to SizeOfMenu do       {Notice only print what's needed}
            if a <> highlight then                {Don't highlight it}
               writeln('   ', choices[a])
            else {A = Highlight}                  {highlight it}
            begin
               write('   ');                      {for asthetic purposes}
               textbackground(white);
               textcolor(blue);
               writeln(choices[a]);
               textcolor(green);                  {choices are in green}
               textbackground(blue);
            end; {else}
         gotoxy(lo(windmax), hi(windmax));     {await choice}
         setupwindow;                          {return window to normal}
      end; {ShowChoices}

   begin {ShowMenu}
      clrscr;
      ShowTitle;
      ShowChoices(highlight);
   end; {ShowMenu}

begin {Program}
   setupwindow;
   SetupThings(leave, choices, highlight);
   repeat
      showmenu(highlight);
      pauser := 0;
      repeat
         if not keypressed then
         begin
            inc(pauser);
            delay(1);
         end {if}
         else
         begin                       {Used for screenSaver}
            selection := upcase(readkey);
            pauser := ScreenSaveTime+1; {Don't do screenSaver, reset}
            ExeChoice(selection, highlight, leave);
         end; {else}
      until pauser >= ScreenSaveTime;   {loop exited, ScreenSaver? or exe?}
      if pauser = ScreenSaveTime then
         ScreenSaver;                   {wait for a while, if nothing...}
      {else do repeat loop}
   until leave;                         {Only way to exit menu}
end.
