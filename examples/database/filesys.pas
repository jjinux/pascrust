{t                                      Shannon Behrens and Mang Tran
                                        Computer Science II
                                        Project IV
                                        5/12/97

   This is the main program for proj4.  Proj4 is basically a filling system
for records of type people.  The people's files are stored in MasterFile.
They are catalogued in KeyFile.  When the program is running KeyFile is stored
in a BST.  FileOps does all the file operations that include deleting records
from the files, altering them, etc.  Proj4 relies on these files:

      BSTUnit : a binary search tree unit programmed in O.O. code.  This is a
         "quiet" unit in that it involves no user interaction.
            Create   IsEmpty     Insert     Delete    Search    Traverse
            FirstItem
      FileOpsUnit : a unit that handles any file operations, including those
         operations that involve transferring data from files into the BST.
            OpenFiles    CloseFiles     InsertPeople    DeletePeople
            AlterPeople  MergeFiles     TreeBuilder     TreeRemoval
            RawTransfer  RewriteFiles

      MasterFile : An unordered binary master file of records.  Records are
         of the type People.

      KeyFile : An binary file or records in ascending order by Peoplekey.ID
         records are of type PeopleKey.  This file is used as a "card
         catalogue" for the MasterFile.}

PROGRAM FILESYS;

USES BSTUnit, FOUnit, CRT, DOS;

VAR
   tree : BST;                  {These are two objects that contain the tree}
   FileSystem : FileOps;        {and masterfile\keyfile respectively}
   choice : char;               {users choice at main menu}
   quit : boolean;              {choice to leave the program}
   AnyFilesOpened : boolean;    {gives user choice of master and key files}

Procedure SetupScreen;
   begin
      textbackground(blue);
      textcolor(yellow);
   end; {SetupScreen}

Procedure MainSetup(var quit:boolean ; var AnyFilesOpened:boolean ;
   var tree:BST);
   begin
      quit:= false;
      AnyFilesOpened:= false;
      tree.create;
   end; {MainSetup}

Procedure Showtitle;
   begin
      writeln;
      writeln('                      Welcome to Shannon and Mang''s Database');
      gotoxy(1,hi(windmax));
      writeln('Please choose:');
   end; {showtitle}

Procedure ShowMenu(anyFilesOpened:boolean);
   type choicesType = array['A'..'H'] of string[50];
   var choices:ChoicesType;
       a: char;
   Procedure SetupChoices(var choices:ChoicesType);
      begin
         choices['A']:='         A.  Create a new file';
         choices['B']:='         B.  Open a file';
         choices['C']:='         C.  Close Current File';
         choices['D']:='         D.  Edit Files';
         choices['E']:='         E.  Raw File Transfer';
         choices['F']:='         F.  Empty Out Current File';
         choices['G']:='         G.  Merge Two Files';
         choices['H']:='         H.  Exit';
      end; {SetupChoices}
   procedure FilesOpened(choices:ChoicesType ; a:char);
      begin
         case a of
         'C','D','F','H','E'    : begin
                                  textcolor(yellow);
                                  writeln(choices[a]);
                                  end;
         else                     begin
                                  textcolor(green);
                                  writeln(choices[a]);
                                  end;
         end; {case}
      end; {WriteYellow}
  procedure FilesNotOpened(choices:Choicestype ; a:char);
      begin
         case a of
            'C','D','F','E'        : begin
                                     textcolor(green);
                                     writeln(choices[a]);
                                     end;
            else                     begin
                                     textcolor(yellow);
                                     writeln(choices[a]);
                                     end;
         end; {case}
      end; {WriteGreen}
   begin {ShowMenu}
      gotoxy(1,8);
      SetupChoices(choices);
      for a:= 'A' to 'H' do
         if AnyFilesOpened then         {Write available choices in yellow,}
            FilesOpened(choices,a)      {and unavailable ones in green}
         else FilesNotOpened(choices,a);
      textcolor(yellow);
      gotoxy(16,hi(windmax));           {Prepare for input}
   end; {showMenu}

Procedure Whichfile(var filename:pathstr ; NewOrOpen:integer);
   var drive:char;
   Procedure DealWithPossibilities(var filename:pathstr ;
      var drive:char);
      begin
         if pos('.',filename)<>0 then      {get rid of extensions}
            delete(filename, pos('.',filename),
               length(filename)-pos('.',filename)+1);
         if pos(':',filename)=0 then
            begin
            write('           Which drive?  ');
            drive:= readkey;
            filename:= drive+':'+filename;
            writeln;
            end; {if}
      end; {DealWithPossibilities}
   Procedure SetupScreen(NewOrOpen:integer);
      begin
         clrscr;
         gotoxy(1,7);
         if NewOrOpen=0 then
            begin
            writeln('           Please enter the full path and name of the file');
            writeln('           which you wish to create.');
            end {if}
         else
            writeln('           Please enter the name of the file you wish to open.');
         gotoxy(12,wherey+1);
      end; {SetupScreen}
   Procedure TellResults(NewOrOpen:integer ; fileName:pathstr);
      begin
         write('           Attempting to ');
         if NewOrOpen=0 then
            write('create ')
         else write('open ');
         write(filename);
      end; {TellResults}
   begin {WhichFile}                    {newOrOpen: 0:new 1:open}
      SetupScreen(NewOrOpen);
      textcolor(green);
      readln(filename);
      textcolor(yellow);
      DealWithPossibilities(filename,drive);
      TellResults(NewOrOpen,fileName);
   end; {WhichFile}

Procedure NewFile(var tree:BST ; var FileSystem:FileOps ; var AnyfilesOpened:
   boolean);
   var FileName:pathstr;
       tryagain:char;
   {Procedure DealWithErrors(var TryAgain:char);
      begin
         write('            ');            just in case of Doserror
         case Doserror of
            2,0: great;
            3 : writeln('Error:  Path not found.');
            5 : writeln('Error:  Access denied.');
            8 : writeln('Error:  Not enough memory.');
           10 : writeln('Error:  Invalid environment.');
           11 : writeln('Error:  Invalid format');
           else writeln;
         end; case
         writeln('           File creation unsuccessful, would you like to try again?');
         write('              (y or n)  ');
         tryagain:= upcase(readkey);
      end; TellAboutErrors  }
   begin {NewFile}
      tryagain:= 'N';
      whichfile(FileName,0);            {0 denotes get new file}
      filename:= fexpand(filename);
      filesystem.MasterfileAddress:= filename+'.dat';
      filesystem.KeyFileAddress:= filename+'.key';
      filesystem.Openfiles(0);
      writeln;writeln;writeln('           File Created.');
      anyfilesOpened:= true;
      repeat until keypressed;             {at this point, new file is open}
   end; {NewFile}                          {and the BST is created}

Procedure POpenFile(var tree:BST ; var FileSystem:FileOps ; var
   AnyFilesOpened:boolean);
   var FileName:pathstr;
       TryAgain:char;
   Procedure FixUpFileName(var filename:pathstr);
      begin
         filename:= fexpand(filename);
         filename:= Fsearch(FileName+'.dat', 'a:;c:;h:');
      end; {FixUpFileName}
   Procedure SomeThingWrong(var Tryagain:char);
      begin
         writeln; writeln;
         writeln('           File not found.  Try typing in just the name.');
         writeln('           Would you like to try again?');
         write('               (y or n)  ');
         TryAgain := upcase(readkey);
      end; {SomethingWrong}
   Procedure OpenthemFiles(var FileSystem:fileOps ; var filename:pathstr ;
      var AnyFilesOpened:boolean);
      begin
         delete(filename, pos('.',filename),   {get rid of .dat so that we can}
            length(filename)-pos('.',filename)+1);       {put in .dat or .key}
         FileSystem.MasterFileAddress:= filename+'.dat';
         FileSystem.KeyfileAddress:= filename+'.key';
         FileSystem.OpenFiles(1);
         AnyfilesOpened:= true;
         writeln; writeln; writeln('           File opened.');
         repeat until keypressed;
       end; {OpenThemFiles}
   begin {POpenFile}
      repeat
         tryAgain:= 'N';
         WhichFile(FileName,1);            {1 denotes get existing file}
         FixUpFileName(fileName);
         if filename = '' then
            SomethingWrong(TryAgain)
         else
            begin
            OpenThemfiles(FileSystem,filename,AnyfilesOpened);
            FileSystem.treeBuilder(tree);  {At this point, we have hopefully}
            end; {else}
      until tryagain<>'Y';                 {Opened a file and filled up the}
   end; {POpenFile}                        {BST}

Procedure CloseCurrentFile(var tree:BST ; var FileSystem:FileOps ; var
   anyfilesOpened:boolean);
   begin
      filesystem.treeRemoval(tree);
      filesystem.closefiles;
      anyfilesOpened:= false;
      textcolor(yellow+blink);
      write(' File Saved.');
      textcolor(yellow);
      repeat until keypressed;
      gotoxy(wherex-12,wherey);
      write('            ');            {erase ' file closed. '}
      gotoxy(wherex-12,wherey);
   end; {CloseCurrentFile}

Procedure FileEditor(var tree:BST ; var FileSystem:FileOps);
   var choice:char;
   Procedure ShowChoices;
      begin
         clrscr;
         gotoxy(1,7);
         writeln('            A. Insert a record');
         writeln('            B. Delete a record');
         writeln('            C. Edit an existing record');
         gotoxy(lo(windmax)-14,hi(windmax));      {Lower right hand corner}
         writeln('Hit X to exit');
      end; {Showchoices}
   Procedure ShowTitle;
      begin
         gotoxy(1,1);
         writeln;
         writeln('                           Welcome to File Editor');
         gotoxy(1,hi(windmax));
         write('Please choose:  ');     {cursor is ready for input}
      end; {ShowTitle}
   Procedure InsertDeleteEditTitle(which:byte);
      begin                             {0:insert ; 1:delete ; 3:edit}
         gotoxy(1,12);
         writeln('            Please enter the ID number of the');
         write('            person you wish to ');
         case which of
            0 : write('insert:  ');
            1 : write('delete:  ');
            2 : write('edit:  ');
         end; {case}
      end; {InsertDeleteEditTitle}
   Procedure InsertRecord(var tree:BST ; var FileSystem:FileOps);
      var item:people;
          successful:boolean;
      begin
         InsertDeleteEditTitle(0);      {0 signifies we are inserting}
         readln(Item.Id);
         writeln;write('            Person''s name?  ');
         Readln(Item.name);
         write('            Index?  ');        
         Readln(Item.Index);
         FileSystem.InsertPeople(item,successful,tree);
         writeln;                       {Inserts into tree and into files}
         if successful then
            writeln('            Person inserted.')
         else writeln('            Person already inserted!');
         repeat until keypressed;
      end; {InsertRecord}
   Procedure DeleteRecord(var tree:BST ; var FileSytem:FileOps);
      var item:people;
          successful:boolean;
      begin
         InsertDeleteEditTitle(1);      {1 signifies we are deleting}
         readln(Item.ID);
         FileSystem.DeletePeople(item,successful,tree);
         writeln;                       {Deletes from tree and from files}
         if successful then
            writeln('            Person deleted.')
         else writeln('            Person not on list!');
         repeat until keypressed;
      end; {DeleteRecord}
   Procedure EditRecord(var tree:BST ; var FileSystem:FileOps);
      const First = 6;                  {For use with gotoxy's}
            Second = 17;
            Third = 45;
      var item,newItem:people;
          successful:boolean;
          confirm:char;
      Procedure WriteOutInfo(item:people);
         begin
            writeln; writeln;
            writeln('     ID         Name                        Index');
            gotoxy(First,wherey); write(item.ID);
            gotoxy(Second,wherey); write(item.name);
            gotoxy(Third,wherey); write(item.Index:0:4);
         end; {WriteOutInfo}
      Procedure GetNewItem(var Newitem:people);
         begin
            writeln; writeln;
            Write('Please enter new ID:');
            gotoxy(First,wherey+2); readln(Newitem.ID);
            gotoxy(1,wherey-3); write('Please enter new Name:        ');
            gotoxy(Second,wherey+2); readln(Newitem.Name);
            gotoxy(1,wherey-3); write('Please enter new Index:       ');
            gotoxy(Third,wherey+2); readln(Newitem.Index);
            gotoxy(1,wherey-3); write('Do you want to save these changes?  ');
         end; {GetNewItem}
      Procedure ConfirmedEditing(var item:People ; var successful:boolean ;
         var FileSystem:FileOps ; var tree:BST ; var NewItem:People);
         begin                    {we know item is there to delete};
            Filesystem.DeletePeople(item,successful,tree);
            if not successful then
               begin
               writeln('Error:  See FileSys Edit Record');
               readln;
               end; {if}
            Filesystem.InsertPeople(NewItem,successful,tree);
            if not successful then
               begin
               writeln('That person is already iserted!');
               repeat until keypressed;
               end; {if}
         end; {ConfirmedEditing}
      Procedure SayNotFound;
         begin
            writeln;
            writeln('            Item not in file!!!!');
            repeat until keypressed;
         end; {SayNotFound}
      begin {EditRecord}
         InsertDeleteEditTitle(2);      {2 signifies we are editing}
         readln(Item.ID);
         FileSystem.AlterPeople(item,successful,tree);  {Gives info on item,}
         if successful then
            begin
            writeOutInfo(item);      {AlterPeople does'nt actually change it.}
            GetNewItem(NewItem);
            repeat
               confirm:= upcase(readkey);
               if confirm='Y' then
                  ConfirmedEditing(item,successful,Filesystem,tree,NewItem);
            until (confirm='Y') or (confirm='N') or (confirm=chr(27));
            end {if}
         else SayNotFound;
      end; {EditRecord}

   Procedure ExeChoice(choice:char ; var tree:BST ; var FileSystem:Fileops);
      begin
         case choice of
            'A' : InsertRecord(tree,FileSystem);
            'B' : DeleteRecord(tree,FileSystem);
            'C' : EditRecord(tree,FileSystem);
         end; {case}
      end; {ExeChoice}
   begin {FileEditor}
      repeat
         ShowChoices;              {contains a clrscr}
         ShowTitle;                {This includes promt to choose}
         choice:= upcase(readkey);
         ExeChoice(choice,tree,FileSystem);
      until (choice = 'X') or (choice = chr(27));
   end; {FileEditor}

Procedure RawFileTrns(var Into:FileOps ; var tree:BST);
   var From:FromType;
       NotFound:boolean;
       Tryagain:char;
   {Founit needs Into (already opened) and From (unOpened).  From (the text)
    will be transfered into Into (Fileops).  Into will be erased before this
    transfer.  The most important thing here is to get the right filename
    for From and then call Into.RawTransfer}
   Procedure SetupTitle;
      begin
         clrscr;
         writeln;
         writeln('                        Welcome to Raw File Transfer');
         writeln;
         writeln;
      end; {Setuptitle}
   Procedure DealWithPossibilities(var FromAddress:filestring ;
      var NotFound:boolean);
      var temp:PathStr;
      begin
           if pos(':',FromAddress)=0 then
              begin
              gotoxy(13,wherey+1); write('Which Drive?  ');
              FromAddress:= upcase(readkey) + ':' + FromAddress;
              end; {if}
           FromAddress:= Fexpand(FromAddress);
           Temp := FSearch(FromAddress,'A:; A:\; c:\; c:; h:; h:\');
           If Temp = '' then
               NotFound:= true
           else
               NotFound:= false;
      end; {DealWithPossibilities}
   Procedure GetFrom(var from:FromType ; var NotFound:boolean);
      begin
         writeln;
         writeln('            Please enter the filename of the Raw File.');
         textcolor(green);
         gotoxy(13,wherey+1);
         readln(From.MasterTextAddress);
         textcolor(yellow);
         DealWithPossibilities(From.MasterTextAddress,NotFound);
      end; {GetFrom}
   Function Confirm:boolean;
      var choice:char;
      begin
         writeln; writeln;
         writeln('            Are you sure you want to do this?  (y or n)');
         writeln('            The current file will be erased in order');
         write('            to hold the incoming information:  ');
         repeat
            choice:= upcase(readkey);
            if choice = 'Y' then Confirm:= true
            else if (choice = 'N') or (choice = chr(27)) then
               confirm:= false;
         until (choice='Y') or (choice='N') or (choice=chr(27));
         writeln;
      end; {Confirm}
   Procedure TellEmItHappened;
      begin
         gotoxy(wherex,wherey-2);
         writeln('            Raw file transfer successful!');
         repeat until keypressed;
      end; {TellEmItHappened}
   Procedure TellEmItDidnt;
      begin
         writeln;
         writeln('            Raw file transfer aborted.');
         repeat until keypressed;
      end; {TellEmItDidnt}
   Procedure TellEmNotFound;
      begin
         writeln;
         writeln('            File not found.  Try just typing in the name.');
         write('            Would you like to try again? (y or n)  ');
      end; {TellEmNotFound}
   begin {RawFileTrns}
      Repeat
         Tryagain:= 'N';
         SetupTitle;
         GetFrom(from,NotFound);
         if Notfound then
            begin
            TellEmNotFound;
            Tryagain:= upcase(readkey);
            end
         else if confirm{function} then
                 begin
                 Into.RawTransfer(From);
                 Into.TreeBuilder(tree);  {put file's info into a tree}
                 TellEmItHappened;
                 end
              else TellEmItDidnt;
      until (tryagain='N') or (tryagain=chr(27));
   end; {RawFileTrns}

Procedure EmptyOutFile(var tree:BST ; var FileSystem:FileOps);
   var choice:char;
   begin                        {x=16 is the place to be for writing}
      textbackground(white);
      textcolor(red);
      write('Are you sure?  Information in file will be lost! (y or n):  ');
      choice:= upcase(readkey);
      if choice = 'Y' then
         begin
         FileSystem.Rewritefiles(tree);    {will also take down the tree}
         gotoxy(16,wherey);
         write('                                                            ');
         gotoxy(16,wherey);
         write('File Erased.');
         repeat until keypressed;
         end; {if}
      textcolor(yellow);
      textbackground(blue);
      gotoxy(16,wherey);
      write('                                                            ');
   end; {EmptyOutFile}

Procedure MergeTwoFiles(var tree:BST ; var FileSystem:FileOps ;
   var AnyfilesOpened:boolean);
   {files A and B will be merged into FileSystem}
   var A,B:FileOps;
       WannaExit:boolean;

   Procedure SetupTitle;
      begin
         clrscr;
         writeln;
         writeln('                          Welcome to File Merge');
         writeln;
         writeln;
      end; {Setuptitle}

   Procedure GetFiles(var A,B,FileSystem:FileOps ; var WannaExit:boolean);
      Procedure DealWithPossibilities(var A:FileOps ; WhichFile:fileString ;
         var NotFound:boolean);
      var drive:char;
          temp:pathstr;
      begin
         if pos('.',WhichFile)<>0 then      {get rid of extensions}
            delete(WhichFile,pos('.',WhichFile),
               length(WhichFile)-pos('.',WhichFile)+1);
         if pos(':',WhichFile)=0 then
            begin
            write('            Which drive?  ');
            drive:= readkey;
            WhichFile:= drive + ':' + WhichFile;
            writeln;
            end; {if}                       {put correct extensions in}
         A.MasterFileAddress:= Fexpand(WhichFile + '.dat');
         A.KeyFileAddress:= Fexpand(WhichFile + '.key');
         Temp := FSearch(A.MasterFileAddress,'A:; A:\; c:\; c:; h:; h:\');
         If Temp = '' then
            NotFound:= true
         else
            NotFound:= false;
      end; {DealWithPossibilities}

      Procedure Ask(which:string);
         begin
            writeln;
            write('            Please enter the filename of the ');
            write(which, ' file:');
            textcolor(green);
            gotoxy(13,wherey+2);
         end; {Ask}

      Procedure SubGetFiles(var A:FileOps ; var WannaExit:boolean ;
         which:string);                   {Which=first,second,or output}
         var NotFound:boolean;
             Continue:char;
             WhichFile:FileString;
         begin
            repeat
               ask(which);
               readln(WhichFile);
               textcolor(yellow);
               DealWithPossibilities(A,WhichFile,NotFound);
               if NotFound then
                  begin
                  writeln;
                  write('            File not found, do you want to continue? (y or n)  ');
                  repeat
                     continue:= upcase(readkey);
                     if (continue = 'N') or (continue = chr(27)) then
                        WannaExit:= true
                     else if Continue = 'Y' then
                        WannaExit:= false;
                  until (Continue='N') or (Continue='Y') or (Continue=chr(27));
                  writeln;
                  end {if}
               else WannaExit:= false;
            until WannaExit or (Notfound=false);
         end; {SubGetFiles}

      begin {GetFiles}
         WannaExit:= false;             {May change in SubGetFiles}
         SubGetFiles(A,WannaExit,'first');
         If not WannaExit then
            SubGetFiles(B,WannaExit,'second');
         If not WannaExit then
            SubGetFiles(FileSystem,WannaExit,'output');
         {At this point if WannaExit is still = false, then continue merge
          procedure}
      end; {GetFiles}

   Function Confirm:boolean;
      var
         Choice:char;
      begin
         writeln;
         writeln('           Are you sure that you want to do this?  (y or n)');
         writeln('           Doing so will destroy any information that is');
         write('           currently in the output file.  ');
         repeat
            choice:= upcase(readkey);
            if choice = 'Y' then confirm:= true
            else if (choice = 'N') or (choice = chr(27)) then
               confirm:= false;
         until (choice='Y') or (choice='N') or (choice=chr(27));
      end; {Confirm}

   Procedure TellEmItHappened;
      const tab=23;
      begin
         gotoxy(1,6);
         textcolor(white+blink);
         textbackground(red);
         gotoxy(tab,wherey); writeln('                              ');
         gotoxy(tab,wherey); writeln('                              ');
         gotoxy(tab,wherey); writeln('         MergeFiles           ');
         gotoxy(tab,wherey); writeln('        Successful!!!         ');
         gotoxy(tab,wherey); writeln('                              ');
         repeat until keypressed;
         textcolor(yellow);
         textbackground(blue);
      end; {TellEmItHappened}

   Procedure TellEmItDidnt;
      begin
         writeln;
         writeln('            Merge files aborted.');
         repeat until keypressed;
      end; {TellEmItDidnt}

   begin {MergeTwoFiles}
      SetupTitle;
      GetFiles(A,B,FileSystem,WannaExit);{WannaExit 'cause can't find file}
      if not WannaExit and confirm{function} then
         begin
         Filesystem.MergeFiles(A,B);
         FileSystem.TreeBuilder(tree);  {put up the tree using files}
         AnyFilesOpened:= true;         {Merged file is}
         TellEmItHappened;
         end
      else TellEmItDidnt;
   end; {MergeTwoFiles}

Procedure ExeChoice(choice:char ; var tree:BST ; var FileSystem:FileOps ;
   var quit:boolean ; var anyFilesOpened:boolean);
   begin
      if anyfilesopened then
         case choice of
            'C' : CloseCurrentFile(tree,FileSystem,AnyFilesOpened);
            'D' : FileEditor(tree,FileSystem);
            'E' : RawFileTrns(FileSystem,tree);
            'F' : EmptyOutFile(tree,FileSystem);
           {'G' : MergeTwoFiles(tree,FileSystem); --> not while file open}
            'H',chr(27): quit:= true;
            'T' : Tree.traverse;
         end {case}
      else {no files opened}
         case choice of
            'A' : NewFile(tree,FileSystem,AnyfilesOpened);
            'B' : POpenFile(tree,FileSystem,AnyfilesOpened);
           {'E' : RawFileTrns(FileSystem); --> open file first}
            'G' : MergeTwoFiles(tree,FileSystem,AnyfilesOpened);
            'H',chr(27): quit:= true;
            'T' : Tree.traverse;
         end; {case}
   end; {ExeChoice}

Procedure Goodbye(var tree:BST ; var FileSystem:FileOps ;
   anyFilesOpened:boolean);
   begin
      writeln('Goodbye');
      if AnyFilesOpened then
         begin
         FileSystem.treeRemoval(tree);
         FileSystem.CloseFiles;
         end; {if}
   end; {Goodbye}

begin
   clrscr;
   SetupScreen;
   MainSetup(quit,AnyFilesOpened,tree);
   repeat
      clrscr;
      showtitle;
      showmenu(anyFilesOpened);
      choice:= upcase(readkey);
      Exechoice(choice,tree,FileSystem,quit,anyfilesopened);
   until quit;
   goodbye(tree,FileSystem,AnyFilesOpened);
end.