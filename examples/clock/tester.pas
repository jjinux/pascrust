{t                              Shannon Behrens
                                7/18/97

Program Tester.  This is the executable version of the clock type.  Please see
Clock for more details.}

Program Tester;
uses Graph,clock,Menu,Crt;
const ClockSize = 1;
var MyClock: ClockType;
    key: char;
Procedure InitializeGraphics;
   var grDriver: Integer;
       grMode: Integer;
   begin
      grDriver := Detect;
      InitGraph(grDriver, grMode,'a:\tictac');
   end; {InitializeGraphics}
Procedure Info;
   var MyMenu: MenuType;
   begin
      MyMenu.create(1,1);
      MyMenu.ChangeTitle('About Vector Clock');
      MyMenu.AddOption('t');
      MyMenu.AddOption(' ');
      MyMenu.AddOption('Vector Clock was programmed by Shannon');
      MyMenu.AddOption('"JJ" Behrens in Fayetteville, NC on 7/18/97.');
      MyMenu.AddOption(' ');
      MyMenu.AddOption('Please send questions and comments to');
      MyMenu.AddOption('210 Lofton Dr., Fayetteville, NC 28311.');
      MyMenu.WriteAsText;
      Repeat until keypressed;
   end; {Info}
begin
   InitializeGraphics;
   MyClock.create(ClockSize);
   repeat
      MyClock.Update;
      if keypressed then
         begin
         key:= upcase(readkey);
         if key = 'I' then
            begin info; MyClock.Create(ClockSize); end;
         end; {if}
   until (key=chr(27)) or (key='E');
   CloseGraph;
end.