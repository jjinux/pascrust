{t                              Shannon Behrens

This program accepts chars and types them onto the screen in two lines.  In
the first line, what is typed is written.  In the second line the encoded/
decoded version of what is typed is written.  This program uses Gina-Marie
Ardizzone's code}

program GMsCode;

uses CRT;

var
   ch    : char;                              {char for encoding}
   a     : integer;                           {pos of ch in string}
   leave : boolean;                           {activated by esc.}

Function GMswitcher(ch : char) : char;
   {encodes/decodes one char}
   begin
      case ch of
         'a'..'m'     :     ch:=chr(ord('z')-(ord(ch)-ord('a')));
         'n'..'z'     :     ch:=chr(ord('a')+(ord('z')-ord(ch)));
         'A'..'M'     :     ch:=chr(ord('Z')-(ord(ch)-ord('A')));
         'N'..'Z'     :     ch:=chr(ord('A')+(ord('Z')-ord(ch)));
         else ch:= ch;
      end; {case}
      GMswitcher:= ch;
   end; {switcher}

procedure TypeColor;
   {textcolor for what is typed}
   begin
      textcolor(yellow);
   end; {TypeColor}

procedure CodeColor;
   {textcolor for what is coded}
   begin
      textcolor(green);
   end; {CodeColor}

procedure Setup(var leave:boolean);
   {Setup and Welcome}
   begin
      clrscr;
      textbackground(blue);
      textcolor(yellow);
      leave:= false;
      writeln('                   Welcome to Gina-Marie''s Code Translator');
      writeln;
   end; {Setup}

function goodchar(ch:char) : boolean;
   {returns true if the char entered is a valid character.  This eliminates
    a lot or problems}
   begin
      case ch of
         chr(32)..chr(126) : GoodChar:= true;
         else GoodChar:= false;
      end; {case}
   end; {Goodchar}

procedure PrintChar(ch:char ; var leave:boolean);
   {prints char:  encoded/decoded and regular.  Prints what is typed in yellow
    and what is encoded/decoded in green below that.  This is the only place
    where leave may be set to true}
   begin
      if goodchar(ch) then
         begin
         write(ch);
         CodeColor;
         gotoxy(wherex-1,wherey+1);
         write(GMswitcher(ch));
         gotoxy(wherex,wherey-1);
         TypeColor;
         end
      else
         case ch of
            chr(8) :  begin                                {backspace}
                         if wherex>1 then                  {back up one space}
                            begin
                            gotoxy(wherex-1,wherey);
                            PrintChar(chr(32),leave);
                            gotoxy(wherex-1,wherey);
                            end {if}
                         else gotoxy(lo(windmax),wherey-2);{go up a line}
                      end; {chr(8)}
            chr(9) :  gotoxy(wherex+5,wherey);             {horiz. tab}
            chr(13):  gotoxy(1,wherey+2);                   {return}
            chr(27):  leave:= true;                        {esc.}
         end; {case}
   end; {PrintChar}

Procedure WrapAround(ch:char);
   begin
      if ((ch=chr(32)) and (wherex>70))
         or (wherex>89)
         then gotoxy(1,wherey+2);
   end; {WrapAround}

procedure CheckForGoodbye(var leave:boolean);
   {This confirms that the user wishes to leave and wishes him farewell.}
   var ch:char;
       currx,curry:integer;             {just in case they want to stay}
   begin
       currx:= wherex; curry:= wherey;
       gotoxy(1,hi(windmax)-1);
       textcolor(blue);
       textbackground(white);
       writeln('Are you sure that you want to exit?');
       textbackground(blue);
       typecolor;
       write('(y or n)  ');
       textbackground(blue);
       ch:= upcase(readkey);
       if ch= 'Y' then                  {wish goodbye}
          begin
             clrscr;
             gotoxy(1,10);
             textcolor(yellow+blink);
             write('                         Thank You For Using GM''sCode!!!');
             typecolor;
             delay(3000);
          end {if}
       else                             {erase question, and return to curr's}
          begin                         {also, set leave:= false}
             gotoxy(1,wherey-1);
             writeln('                                   ');
             write('                                     ');
             gotoxy(currx,curry);
             leave:= false;
          end {else}
   end; {CheckForGoodBye}

begin {program}
   Setup(leave);
   repeat
      ch:= readkey;
      PrintChar(ch,leave);
      WrapAround(ch);
      if leave then CheckForGoodbye(leave);
   until leave;
end.