{t

      FileOpsUnit : a unit that handles any file operations, including those
         operations that involve transferring data from files into the BST.
            OpenFiles    CloseFiles     InsertPeople    DeletePeople
            AlterPeople  MergeFiles     TreeBuilder     TreeRemoval
            RawTransfer  RewriteFiles
      It often refers to BSTUnit.  Please note that this is a quiet unit.
         It does not invoke interaction from the user.  For more info, please
         see filesys.pas}

unit FOUnit;

interface
   uses BSTUnit, Crt, Dos;
   type
      FileString = string[50];
      people = record
         name: string[30];
         ID: LongInt;
         Index: real
      end; {record}
      FromType = record
         MasterText: Text;
         MasterTextAddress: FileString;
      end; {Record}
      MasterFileType = file of people;
      KeyFileType = file of PeopleKey;
      FileOps = object
         MasterFile: MasterFileType;
         KeyFile: KeyFileType;
         MasterFileAddress: FileString;
         KeyFileAddress: FileString;
         procedure OpenFiles(NewOrOpen: integer);
         procedure CloseFiles;
         procedure InsertPeople(item: people; var successful: boolean;
            var tree: BST);
         procedure DeletePeople(item: people; var successful: boolean;
            var tree: BST);
         procedure AlterPeople(var item: people; var successful: boolean;
            var tree: BST);
         procedure MergeFiles(var a, b: FileOps);
         procedure TreeBuilder(var tree: BST);
         procedure TreeRemoval(var tree: BST);
         procedure RawTransfer(var From: FromType);
         procedure RewriteFiles(var tree: BST);
      end; {object}

implementation
   procedure FileOps.OpenFiles(NewOrOpen: integer);
      var temp: pathstr;
      begin                         {NewOrOpen=> 0:newfile   1:existing}
         assign(MasterFile, MasterFileAddress);
         assign(KeyFile, KeyFileAddress);
         if NewOrOpen = 0 then      {you can't reset a nonexisting file}
         begin
            temp := FSearch(MasterFileAddress, 'a:;,c:;h:');
            if temp <> '' then
               writeln(chr(13), chr(10), '           File already exists!!!')
            else
            begin rewrite(MasterFile);
               rewrite(KeyFile);
            end;
         end {if}
         else if NewOrOpen = 1 then {reset, else you'll destroy data}
         begin
            reset(MasterFile);
            reset(KeyFile);
         end;
      end; {.OpenFiles}

   procedure FileOps.CloseFiles;
      begin
         close(MasterFile);
         close(KeyFile);
      end; {.CloseFiles}

   procedure FileOps.InsertPeople(item: people; var successful: boolean;
      var tree: BST);
      {This proc. should insert people into both the tree and into the files}
      var KeyItem: PeopleKey;                   {id,location,before,after}
      begin
         seek(MasterFile, filesize(MasterFile));
         KeyItem.Location := filepos(MasterFile);
         KeyItem.ID := item.ID;
         tree.Insert(KeyItem, successful);      {will check if ID is in tree,}
         if successful then                     {then insert}
            write(MasterFile, item);
         {else leave, filesys will give appropriate warning}
      end; {.InsertPeople}

   procedure FileOps.DeletePeople(item: people; var successful: boolean;
                                  var tree: BST);
      {This proc. should delete people from both the tree and the files.  This
       is based on item.ID.}
      var KeyItem: PeopleKey;
      procedure Fcopy(var a, b: MasterFileType);
         var item: people;
         begin
            if not EOF(a) then
            begin
               read(a, item);
               write(b, item);
            end; {if}
         end; {for}
      procedure GetTempName(MasterFileAddress: FileString;
         var TempFileAddress: FileString);
         {will get a name for TempFile that is in a place || to MasterFile}
         var a: integer;
         begin
            TempFileAddress := MasterFileAddress;
            a := length(TempFileAddress);
            while (TempFileAddress[a] <> '\') and (TempFileAddress[a] <> ':')
               do dec(a);                     {a now at right place}
            Delete(TempFileAddress, a+1, length(TempFileAddress)-(a+1));
            TempFileAddress := TempFileAddress+'temp4del.dat';
         end; {GetTempName}
      procedure NFileDelItem(position: integer; var MasterFile: MasterFileType;
         MasterFileAddress: string);
         {will create a temp file parallel to masterfile.  Will copy all items
          except item at position.  Then will copy temp back into MasterF'.
          Position is a 0-based record number, as given by FilePos.}
         var TempFileAddress: FileString;
            tempFile: MasterFileType;
            a: integer;
         begin
            GetTempName(MasterFileAddress, TempFileAddress);
            assign(tempFile, TempFileAddress);
            rewrite(tempFile);
            seek(MasterFile, 0);                {copy from Mast'F to temp}
            for a := 0 to (position-1) do
               Fcopy(MasterFile, tempFile);
            seek(MasterFile, position+1);       {skip item to be deleted}
            for a := (position+1) to filesize(MasterFile)-1 do
               Fcopy(MasterFile, tempFile);
            rewrite(MasterFile);                {copy from temp to Mast'F}
            seek(tempFile, 0);
            for a := 1 to filesize(tempFile) do
               Fcopy(tempFile, MasterFile);
            close(tempFile);
            erase(tempFile);                    {now get rid of TempFile}
         end; {NFileDelItem}
      begin {.DeletePeople}
         KeyItem.ID := item.ID;
         tree.Search(KeyItem, successful);      {using Keyitem.ID it will give}
         if successful then                     {Keyitem.loc'.  Also will tell}
         begin                                  {if item is in tree}
            tree.Delete(KeyItem, successful);
            NFileDelItem(KeyItem.Location, MasterFile, MasterFileAddress);
            tree.ShiftLocations(KeyItem.Location); {later records moved up}
         end; {if}
         {else leave, filesys will give appropriate warning}
      end; {.DeletePeople}

   procedure FileOps.AlterPeople(var item: people; var successful: boolean;
      var tree: BST);
      var KeyItem: PeopleKey;
      procedure GetWholeItem(var MasterFile: MasterFileType; var item: people;
         PlaceInFile: LongInt);
         begin
            seek(MasterFile, PlaceInFile);
            read(MasterFile, item);
         end; {GetWholeItem}
      begin
         KeyItem.ID := item.ID;
         tree.Search(KeyItem, successful);
         if successful then              {if found, then item is in file also}
            GetWholeItem(MasterFile, item, KeyItem.Location);
      end; {.AlterPeople}

   procedure FileOps.MergeFiles(var a, b: FileOps);
      {The output file is the FileOps}
      var MergeTree: BST;
      procedure Fcopy(var a, b: MasterFileType);
         var item: people;
         begin
            read(a, item);
            write(b, item);
         end; {FCopy}
      procedure ReWriteMain(var MasterFile: MasterFileType;
         var KeyFile: KeyFileType; MasterFileAddress: FileString;
         KeyFileAddress: FileString);
         begin
            assign(MasterFile, MasterFileAddress);
            assign(KeyFile, KeyFileAddress);
            rewrite(MasterFile);
            rewrite(KeyFile);
         end; {ReWriteMain}
      procedure openMergeFile(var a: FileOps);
         begin
            assign(a.MasterFile, a.MasterFileAddress);
            assign(a.KeyFile, a.KeyFileAddress);
            reset(a.MasterFile);
            reset(a.KeyFile);
         end; {OpenMergeFile}
      procedure FSelectCopy(var b: FileOps; var MasterFile: MasterFileType;
         var MergeTree: BST);
         var item: people;
            KeyItem: PeopleKey;
            successful: boolean;
         begin
            read(b.MasterFile, item);
            KeyItem.ID := item.ID;
            KeyItem.Location := filesize(MasterFile);
            MergeTree.Insert(KeyItem, successful); {if not successful then item}
            if successful then          {is a duplicate; thus don't insert it.}
               write(MasterFile, item)
            else write('');                   {do nothing, this is a duplicate}
         end {FSelectCopy};
      begin
         MergeTree.Create;
         ReWriteMain(MasterFile, KeyFile, MasterFileAddress, KeyFileAddress);
         openMergeFile(a);
         openMergeFile(b);
         while not EOF(a.MasterFile) do       {put A into masterfile}
            Fcopy(a.MasterFile, MasterFile);
         a.TreeBuilder(MergeTree);            {put A in tree}
         while not EOF(b.MasterFile) do       {selectively, no Dbls, put B }
            FSelectCopy(b, MasterFile, MergeTree);
         TreeRemoval(MergeTree);              {into tree and file.  Now build}
      end {.Mergefiles};                      {the keyfile}

   procedure FileOps.TreeBuilder(var tree: BST);
      {This is a "recursive" proc. whose goal is to transfer all the records
       in keyfile into the BST.  It will do so in a manner that will produce
       a balanced tree.  This is dependent upon the fact that keyfile is
       sorted}
      procedure Build(var KeyFile: KeyFileType; first, last: LongInt;
                      var tree: BST);
         var successful: boolean;        {if false, duplicate files exist}
            item: PeopleKey;             {used in bussing}
            mid: LongInt;
         begin
            mid := (last+first) div 2;
            if last<first then
               writeln('')               {last case to happen}
            else if first = filesize(KeyFile) then
               writeln('')     {do nothing, another one of those last cases}
            else                         {insert data at position mid in}
            begin                        {keyfile into tree, then...(recr)}
               seek(KeyFile, mid);
               read(KeyFile, item);
               tree.Insert(item, successful);
               Build(KeyFile, first, mid-1, tree);
               Build(KeyFile, mid+1, last, tree);
            end; {else}
         end; {Build}
      begin {.TreeBuilder}
         Build(KeyFile, 0, filesize(KeyFile), tree);
      end; {.TreeBuilder}

   procedure FileOps.TreeRemoval(var tree: BST);
      {Takes all items out of the tree (deleting as it goes) and put them back
       into Keyfile.  The items will be in order.}
      var KeyItem: PeopleKey;
         successful: boolean;          {only used to match header of bst.del}
      begin
         rewrite(KeyFile);
         while not tree.IsEmpty do
         begin
            tree.FirstItem(KeyItem);
            write(KeyFile, KeyItem);
            tree.Delete(KeyItem, successful);
            if not successful then
               writeln('Error:  trying to delete a nonexisting item.  See Fileops.TreeRemoval');
         end; {TreeRemoval}
      end; {.TreeRemoval}

   procedure FileOps.RawTransfer(var From: FromType);
      var item: people;
         KeyItem: PeopleKey;
         successful: boolean;
         TransferTree: BST;
      procedure OpenRawTransferFiles(var MasterFile: MasterFileType;
         var KeyFile: KeyFileType; var From: FromType);
         begin
            {Into should already be open}
            assign(From.MasterText, From.MasterTextAddress);
            rewrite(MasterFile);
            rewrite(KeyFile);
            reset(From.MasterText);
         end; {OpenRawTransferFiles}
      procedure TransportItem(var From: Text; var Into: MasterFileType);
         var item: people;
         begin
            readln(From, item.ID);
            readln(From, item.name);
            readln(From, item.Index);
            write(Into, item);
         end; {TransportItem}
      procedure PutInTree(var MasterF: MasterFileType;var TransferTree: BST);
         var item: people;
            KeyItem: PeopleKey;
            successful: boolean;
         begin
            read(MasterF, item);
            KeyItem.ID := item.ID;
            KeyItem.Location := filepos(MasterF)-1;
            TransferTree.Insert(KeyItem, successful);
            {if not successful then we have duplicates that will sit in the}
            {the masterfile w/o being in the keyfile.  These duplicates are}
            {the result of the raw file.}
         end {PutInTree};
      begin  {.RawTransfer}
         TransferTree.Create;
         OpenRawTransferFiles(MasterFile, KeyFile, From);
         while not EOF(From.MasterText) do   {raw file --> to.masterfile}
            TransportItem(From.MasterText, MasterFile);
         reset(MasterFile);
         while not EOF(MasterFile) do        {Into.Masterfile --> tree}
            PutInTree(MasterFile, TransferTree);
         TreeRemoval(TransferTree);          {tree --> Into.KeyFile}
      end; {.RawTransfer}

   procedure FileOps.RewriteFiles(var tree: BST);
      {Must empty out files and also tear down the tree}
      var KeyItem: PeopleKey;        {Needed in order to tear down the tree}
         successful: boolean;
      begin
         rewrite(MasterFile);        {empties out the files}
         rewrite(KeyFile);
         while not tree.IsEmpty do   {empties out the tree}
         begin
            tree.FirstItem(KeyItem);
            tree.Delete(KeyItem, successful);
         end; {while}
      end; {.RewriteFiles}

begin
   {setup}
end.
