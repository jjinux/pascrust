{t

   BSTUnit : a binary search tree unit programmed in O.O. code.  This is a
         "quiet" unit in that it involves no user interaction.
            Create   IsEmpty     Insert     Delete    Search    Traverse
            FirstItem
   Please see filesys.pas for more info.}

unit BSTUnit;

interface
   type
      PeopleKeyPointer = ^PeopleKey;
      PeopleKey = record
         ID: LongInt;
         Location: integer;
         Before: PeopleKeyPointer;
         After: PeopleKeyPointer;
      end; {record}
      BST = object
         root: PeopleKeyPointer;
         procedure Create;
         function IsEmpty: boolean;
         procedure Insert(var item: PeopleKey; var successful: boolean);
         procedure Delete(item: PeopleKey; var successful: boolean);
         procedure Search(var item: PeopleKey; var found: boolean);
         procedure Traverse; {for testing purposes only}
         procedure FirstItem(var item: PeopleKey);
         procedure ShiftLocations(removed: integer);
      end; {object}

implementation
   procedure BST.Create;
      begin
         root := nil;
      end; {.Create}

   function BST.IsEmpty: boolean;
      begin
         IsEmpty := root = nil;
      end; {.IsEmpty}

   procedure BST.Insert(var item: PeopleKey; var successful: boolean);
      procedure RecInsert(var root: PeopleKeyPointer; var item: PeopleKey;
         var successful: boolean);
         begin
            if root = nil then
            begin
               new(root);
               root^.ID := item.ID;
               root^.Location := item.Location;
               root^.Before := nil;
               root^.After := nil
            end
            else
               if item.ID<root^.ID then
                  RecInsert(root^.Before, item, successful)
               else if item.ID > root^.ID then
                  RecInsert(root^.After, item, successful)
               else successful := false;         {item in tree}
         end {recInsert};
      begin
         successful := true;                     {may change in recInsert}
         RecInsert(root, item, successful);
      end; {.Insert}

   procedure BST.Delete(item: PeopleKey; var successful: boolean);
      procedure recDelete(var root: PeopleKeyPointer; item: PeopleKey;
         var successful: boolean);
         var TempPtr: PeopleKeyPointer;
         begin
            if root = nil then       {empty tree/subtree, item not found}
               successful := false
            else
               if item.ID<root^.ID then
                  recDelete(root^.Before, item, successful)
               else if item.ID>root^.ID then
                  recDelete(root^.After, item, successful)
               else                  {item found -- delete node}
                  if root^.Before = nil then
                  begin           {no before}
                     TempPtr := root^.After;
                     dispose(root);
                     root := TempPtr;
                  end {if}
                  else if root^.After = nil then
                  begin           {before, but no after}
                     TempPtr := root^.Before;
                     dispose(root);
                     root := TempPtr
                  end {else if}
                  else               {before and after exist}
                  begin
                     TempPtr := root^.After;
                     while TempPtr^.Before <> nil do
                        TempPtr := TempPtr^.Before;
                     {Copy only the successor's data.  Copying the whole
                      record (Root^ := TempPtr^) would also copy its
                      Before/After links and cut the tree apart.}
                     root^.ID := TempPtr^.ID;
                     root^.Location := TempPtr^.Location;
                     recDelete(root^.After, TempPtr^, successful);
                  end; {else}
         end {recDelete};
      begin  {.Delete}
         successful := true;
         recDelete(root, item, successful);
      end; {.Delete}

   procedure BST.Search(var item: PeopleKey; var found: boolean);
      {will search the tree for item, and then put info in tree's item into
       the item to be sent back.  This is necessary in order to find the
       item's position in masterfile.}
      procedure recSearch(var root: PeopleKeyPointer; var item: PeopleKey;
         var found: boolean);
         begin
            if root = nil then                   {empty tree}
               found := false
            else
            begin                 { there is a non empty tree to search}
               if item.ID < root^.ID then           {descend before subtree}
                  recSearch(root^.Before, item, found)
               else if item.ID > root^.ID then      {descend after subtree}
                  recSearch(root^.After, item, found)
               else                                 { ID found}
                  item := root^;
            end {else}
         end {recSearch};
      begin  {.Search}
         found := true;
         recSearch(root, item, found);
      end; {.Search}

   procedure BST.Traverse; {for testing purposes only}
      procedure recTraverse(root: PeopleKeyPointer);
         begin
            if root = nil then
            {do nothing, this is our base case}
            else
            begin
               recTraverse(root^.Before);
               write(root^.ID:8, '  ');
               writeln(root^.Location);
               recTraverse(root^.After);
            end; {else}
         end; {recTraverse}
      begin {traverse}
         writeln;
         if IsEmpty then
            writeln('       TREE IS EMPTY')
         else
         begin
            writeln('ID        Position');
            recTraverse(root);
         end; {else}
         writeln('Hit ENTER key to exit');
         readln;
      end; {BST.Traverse}

   procedure BST.FirstItem(var item: PeopleKey);
      {will return the first item of the BST without changing its value}
      procedure RecFirstItem(var item: PeopleKey; root: PeopleKeyPointer);
         begin
            if (root^.Before = nil) then {base case}
               item := root^
            else RecFirstItem(item, root^.Before);
         end {RecFirstItem};
      begin  {FirstItem}
         if root = nil then
            writeln('Error:  trying to get an item from an empty tree')
         else
            RecFirstItem(item, root);
      end {FirstItem};

   procedure BST.ShiftLocations(removed: integer);
      {After the record at file position Removed is deleted from the master
       file, every later record is one position earlier.}
      procedure RecShift(root: PeopleKeyPointer);
         begin
            if root <> nil then
            begin
               if root^.Location>removed then
                  dec(root^.Location);
               RecShift(root^.Before);
               RecShift(root^.After);
            end; {if}
         end; {RecShift}
      begin {ShiftLocations}
         RecShift(root);
      end; {ShiftLocations}

begin
   {setup}
end.
