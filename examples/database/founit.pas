{t

      FileOpsUnit : a unit that handles any file operations, including those
         operations that involve transferring data from files into the BST.
            OpenFiles    CloseFiles     InsertPeople    DeletePeople
            AlterPeople  MergeFiles     TreeBuilder     TreeRemoval
            RawTransfer  RewriteFiles
      It often refers to BSTUnit.  Please note that this is a quiet unit.
         It does not invoke interaction from the user.  For more info, please
         see filesys.pas}

UNIT FOUnit;

INTERFACE
   USES BSTUnit, CRT, Dos;
   TYPE
      FileString = string[50];
      People = record
                  Name: String[30];
                  ID: LongInt;
                  Index: Real
               end; {record}
      FromType = Record
                    MasterText : Text;
                    MasterTextAddress : FileString;
                 end; {Record}
      MasterFileType = file of People;
      KeyFileType = file of PeopleKey;
      FileOps = object
         MasterFile : MasterFileType;
         KeyFile : KeyFileType;
         MasterFileAddress : FileString;
         KeyFileAddress : FileString;
         Procedure OpenFiles(NewOrOpen:integer);
         Procedure CloseFiles;
         Procedure InsertPeople(item:people ; var successful:boolean ;
            var tree:BST);
         Procedure DeletePeople(item:people ; var successful:boolean ;
            var tree:BST);
         Procedure AlterPeople(var item:People ; var successful:boolean ;
            var tree:BST);
         Procedure MergeFiles(var A,B:FileOps);
         Procedure TreeBuilder(var Tree:BST);
         Procedure TreeRemoval(var Tree:BST);
         Procedure RawTransfer(var From:FromType);
         Procedure RewriteFiles(var tree:BST);
         end; {object}

IMPLEMENTATION
   Procedure FileOps.OpenFiles(newOrOpen:integer);
      var temp : pathstr;
      begin                         {NewOrOpen=> 0:newfile   1:existing}
         assign(MasterFile,MasterFileAddress);
         assign(KeyFile,KeyFileAddress);
         if newOrOpen = 0 then      {you can't reset a nonexisting file}
            begin
            temp:=fsearch(MasterFileAddress,'a:;,c:;h:');
            if temp<>'' then
               writeln(chr(13),chr(10),'           File already exists!!!')
            else
               begin rewrite(Masterfile);
               rewrite(KeyFile);
               end;
            end {if}
         else if NewOrOpen = 1 then {reset, else you'll destroy data}
            begin
            reset(MasterFile);
            reset(keyFile);
            end;
      end; {.OpenFiles}

   Procedure FileOps.CloseFiles;
      begin
         close(MasterFile);
         close(KeyFile);
      end; {.CloseFiles}

   Procedure FileOps.InsertPeople(item:people ; var successful:boolean ;
      var tree:BST);
      {This proc. should insert people into both the tree and into the files}
      var KeyItem:PeopleKey;                    {id,location,before,after}
      begin
         seek(masterfile,filesize(masterfile));
         KeyItem.location:= filepos(masterfile);
         KeyItem.ID:= item.ID;
         tree.Insert(keyitem,successful);       {will check if ID is in tree,}
         if successful then                     {then insert}
            write(masterfile,item);
         {else leave, filesys will give appropriate warning}
      end; {.InsertPeople}

   Procedure FileOps.DeletePeople(item:People ; var successful:boolean ;
                                  var tree:BST);
      {This proc. should delete people from both the tree and the files.  This
       is based on item.ID.}
      var KeyItem:PeopleKey;
      Procedure Fcopy(var A,B:MasterFileType);
         var item:people;
         begin
            if not eof(A) then
               begin
               read(A,item);
               write(B,item);
               end; {if}
         end; {for}
      Procedure GetTempName(MasterFileAddress:FileString ;
         var TempFileAddress:FileString);
         {will get a name for TempFile that is in a place || to MasterFile}
         var a:integer;
         begin
            TempfileAddress:= MasterFileAddress;
            a:= length(TempFileAddress);
            while (TempfileAddress[a]<>'\') and (TempFileAddress[a]<>':')
               do dec(a);                     {a now at right place}
            delete(TempfileAddress,a+1,Length(TempFileAddress)-(a+1));
            TempFileAddress:= TempFileAddress+'temp4del.dat';
         end; {GetTempName}
      Procedure NFileDelItem(Position:integer ; var masterfile:MasterFileType ;
         MasterFileAddress:string);
         {will create a temp file parallel to masterfile.  Will copy all items
          except item at position.  Then will copy temp back into MasterF'}
         var tempFileAddress : Filestring;
             tempFile : MasterFiletype;
             a : integer;
         begin
            GetTempName(MasterFileAddress,tempFileAddress);
            assign(Tempfile,TempFileAddress);
            rewrite(tempfile);
            for a:= 1 to (position-1) do        {copy from Mast'F to temp}
               Fcopy(MasterFile,Tempfile);      {ignore item to be deleted}
            for a:= (position+1) to filesize(MasterFile) do
               Fcopy(MasterFile,Tempfile);
            rewrite(masterfile);                {copy from temp to Mast'F}
            for a:= 1 to filesize(tempFile) do
               Fcopy(TempFile,MasterFile);
            erase(tempFile);                    {now get rid of TempFile}
         end; {NFileDelItem}
      begin {.DeletePeople}
         KeyItem.ID:= item.ID;
         Tree.Search(Keyitem,successful);       {using Keyitem.ID it will give}
         if successful then                     {Keyitem.loc'.  Also will tell}
            begin                               {if item is in tree}
            Tree.Delete(Keyitem,successful);
            NFileDelItem(keyitem.Location,masterfile,MasterFileAddress);
            end; {if}
         {else leave, filesys will give appropriate warning}
      end; {.DeletePeople}

   Procedure FileOps.AlterPeople(var item:People ; var successful:boolean ;
      var tree:BST);
      var keyItem:PeopleKey;
      Procedure GetWholeItem(var MasterFile:MasterFileType ; var item:People ;
         PlaceInFile:longint);
         begin
            seek(MasterFile,PlaceInFile);
            read(MasterFile,Item);
         end; {GetWholeItem}
      begin
         KeyItem.Id:= item.Id;
         Tree.Search(KeyItem,successful);
         if successful then              {if found, then item is in file also}
            GetWholeItem(Masterfile,item,Keyitem.location);
      end; {.AlterPeople}

  Procedure FileOps.MergeFiles(var A,B:FileOps);
     {The output file is the FileOps}
     var MergeTree:BST;
     Procedure Fcopy(var A,B:MasterFileType);
        var item:people;
        begin
           read(A,item);
           write(B,item);
        end; {FCopy}
     Procedure ReWriteMain(var MasterFile:MasterFileType ;
        var KeyFile:KeyFileType ; MasterFileAddress:FileString ;
        KeyFileAddress:FileString);
        begin
           assign(MasterFile,MasterFileAddress);
           assign(KeyFile,KeyFileAddress);
           rewrite(MasterFile);
           rewrite(Keyfile);
        end; {ReWriteMain}
     Procedure OpenMergeFile(var A:FileOps);
        begin
           Assign(A.MasterFile,A.MasterFileAddress);
           Assign(A.KeyFile,A.KeyFileAddress);
           reset(A.MasterFile);
           reset(A.KeyFile);
        end; {OpenMergeFile}
     Procedure FSelectCopy(var B:FileOps ; var MasterFile:MasterFileType ;
        var MergeTree:BST);
        var item:people;
            KeyItem:PeopleKey;
            successful:boolean;
        begin
           read(B.MasterFile,item);
           KeyItem.Id:=item.Id;
           KeyItem.Location:=FileSize(MasterFile);
           mergeTree.insert(keyitem,successful); {if not successful then item}
           if successful then          {is a duplicate; thus don't insert it.}
              write(MasterFile,item)
           else write('');                   {do nothing, this is a duplicate}
        end{FSelectCopy};
     begin
        MergeTree.Create;
        ReWriteMain(MasterFile,KeyFile,MasterFileAddress,KeyFileAddress);
        openMergeFile(A);
        openMergeFile(B);
        while not EOF(A.masterfile) do       {put A into masterfile}
           Fcopy(A.masterfile,masterfile);
        A.TreeBuilder(MergeTree);            {put A in tree}
        while not EOF(B.MasterFile) do       {selectively, no Dbls, put B }
           FSelectcopy(B,MasterFile,MergeTree);
        TreeRemoval(MergeTree);              {into tree and file.  Now build}
     end{.Mergefiles};                       {the keyfile}

  Procedure FileOps.TreeBuilder(var Tree:BST);
      {This is a "recursive" proc. whose goal is to transfer all the records
       in keyfile into the BST.  It will do so in a manner that will produce
       a balanced tree.  This is dependent upon the fact that keyfile is
       sorted}
      Procedure Build(var KeyFile:KeyFileType ; first,last:longint ;
                      var Tree:BST);
         var successful:boolean;         {if false, duplicate files exist}
             item:PeopleKey;             {used in bussing}
             mid:Longint;
         begin
            mid:= (last+first) div 2;
            if last<first then
               writeln('')               {last case to happen}
            else if first=filesize(keyfile) then
               writeln('')     {do nothing, another one of those last cases}
            else                         {insert data at position mid in}
               begin                     {keyfile into tree, then...(recr)}
                  seek(keyfile,mid);
                  read(keyfile,item);
                  tree.insert(item,successful);
                  Build(keyfile,first,mid-1,Tree);
                  Build(keyfile,mid+1,last,Tree);
               end; {else}
         end; {Build}
      begin {.TreeBuilder}
         Build(KeyFile,0,FileSize(keyFile),tree);
      end; {.TreeBuilder}

   Procedure FileOps.TreeRemoval(var Tree:BST);
      {Takes all items out of the tree (deleting as it goes) and put them back
       into Keyfile.  The items will be in order.}
      var keyitem:peopleKey;
          successful:boolean;           {only used to match header of bst.del}
      begin
         Rewrite(Keyfile);
         While Not Tree.IsEmpty do
            begin
            Tree.FirstItem(keyitem);
            write(keyfile,keyitem);
            Tree.Delete(keyitem,successful);
            if not successful then
               writeln('Error:  trying to delete a nonexisting item.  See Fileops.TreeRemoval');
            end; {TreeRemoval}
      end; {.TreeRemoval}
 
   Procedure FileOps.RawTransfer(var From:FromType);
      var item:people;
          KeyItem:peopleKey;
          successful:boolean;
          TransferTree:BST;
      Procedure OpenRawTransferFiles(var Masterfile:MasterFiletype ;
         var KeyFile:KeyFileType ; var From:FromType);
         begin
            {Into should already be open}
            Assign(From.MasterText,From.MasterTextAddress);
            Rewrite(MasterFile);
            Rewrite(KeyFile);
            Reset(From.MasterText);
         end; {OpenRawTransferFiles}
      Procedure TransportItem(var From:Text ; var Into:MasterFileType);
         var item:people;
         begin
            readln(From,Item.ID);
            readln(From,Item.Name);
            readln(From,Item.Index);
            write(Into,Item);
         end; {TransportItem}
      Procedure PutInTree(var MasterF:MasterFileType;var TransferTree:BST);
         var item:people;
             KeyItem:peopleKey;
             successful:boolean;
         begin
            read(MasterF,item);
            KeyItem.Id:=item.Id;
            KeyItem.Location:=FilePos(MasterF)-1;
            TransferTree.Insert(KeyItem,successful);
            {if not successful then we have duplicates that will sit in the}
            {the masterfile w/o being in the keyfile.  These duplicates are}
            {the result of the raw file.}
         end{PutInTree};
      begin {.RawTransfer}
         TransferTree.Create;
         OpenRawTransferFiles(MasterFile,KeyFile,From);
         while not EOF(From.MasterText) do   {raw file --> to.masterfile}
            TransportItem(From.MasterText,MasterFile);
         reset(MasterFile);
         while not EOF(MasterFile) do        {Into.Masterfile --> tree}
            PutInTree(masterFile,TransferTree);
         TreeRemoval(Transfertree);          {tree --> Into.KeyFile}
      end; {.RawTransfer}

   Procedure FileOps.RewriteFiles(var tree:BST);
      {Must empty out files and also tear down the tree}
      var keyitem:PeopleKey;         {Needed in order to tear down the tree}
          successful:boolean;
      begin
         rewrite(Masterfile);        {empties out the files}
         rewrite(keyfile);
         while not tree.isempty do   {empties out the tree}
            begin
            tree.FirstItem(Keyitem);
            tree.Delete(keyitem,successful);
            end; {while}
      end; {.RewriteFiles}

begin
   {setup}
end.
