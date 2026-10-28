{t

   BSTUnit : a binary search tree unit programmed in O.O. code.  This is a
         "quiet" unit in that it involves no user interaction.
            Create   IsEmpty     Insert     Delete    Search    Traverse
            FirstItem
   Please see filesys.pas for more info.}

UNIT BSTUnit;

INTERFACE
   TYPE
       PeopleKeyPointer = ^PeopleKey;
       PeopleKey = record
                      ID:  LongInt;
                      Location:  integer;
                      before :  PeopleKeyPointer;
                      after :  PeopleKeyPointer;
                   end; {record}
       BST = object
          Root : PeopleKeyPointer;
          Procedure Create;
          Function IsEmpty:boolean;
          Procedure Insert(var item:PeopleKey ; var successful:boolean);
          Procedure Delete(item:PeopleKey ; var successful:boolean);
          Procedure Search(var item:PeopleKey ; var found:boolean);
          Procedure Traverse; {for testing purposes only}
          Procedure FirstItem(var item:PeopleKey);
          Procedure ShiftLocations(removed:integer);
       end; {object}

IMPLEMENTATION
   Procedure BST.Create;
      begin
         root:=nil;
      end; {.Create}

   Function BST.IsEmpty:boolean;
      begin
         IsEmpty:= root=nil;
      end; {.IsEmpty}

   Procedure BST.Insert(var item:PeopleKey ; var successful:boolean);
      Procedure RecInsert(var root:PeopleKeyPointer ; var item:PeopleKey ;
         var successful:boolean);
         begin
            if root=nil then
               begin
                  new(root);
                  root^.Id:=item.id;
                  root^.Location:=item.Location;
                  root^.Before:=nil;
                  root^.After:=nil
               end
            else
               if Item.Id<root^.id then
                  recInsert(root^.before,Item,successful)
               else if Item.Id > root^.Id then
                  recInsert(root^.after,item,successful)
               else successful:=false;           {item in tree}
         end{recInsert};
      begin
         successful:=true;                       {may change in recInsert}
         RecInsert(root,item,successful);
      end; {.Insert}

   Procedure BST.Delete(item:PeopleKey ; var successful:boolean);
      Procedure recDelete(var root:PeopleKeyPointer ; item:PeopleKey ;
         var successful:boolean);
         var tempPtr:PeopleKeyPointer;
         begin
            if root=nil then         {empty tree/subtree, item not found}
               successful:=false
            else
               if item.Id<root^.Id then
                  recDelete(root^.Before,item,successful)
               else if item.Id>root^.Id then
                  recDelete(root^.After,item,successful)
               else                  {item found -- delete node}
                  if Root^.Before=nil then
                     begin           {no before}
                        TempPtr:=Root^.After;
                        dispose(root);
                        Root:=TempPtr;
                     end{if}
                  else if Root^.After=nil then
                     begin           {before, but no after}
                        TempPtr:=Root^.Before;
                        dispose(root);
                        Root:=TempPtr
                     end{else if}
                  else               {before and after exist}
                     begin
                        TempPtr:= Root^.After;
                        while TempPtr^.Before<>nil do
                           TempPtr:=TempPtr^.Before;
                        {Copy only the successor's data.  Copying the whole
                         record (Root^ := TempPtr^) would also copy its
                         Before/After links and cut the tree apart.}
                        Root^.ID:= TempPtr^.ID;
                        Root^.Location:= TempPtr^.Location;
                        recDelete(Root^.After,TempPtr^,successful);
                     end;{else}
         end{recDelete};
      begin {.Delete}
         successful:=true;
         recDelete(root,item,successful);
      end; {.Delete}

   Procedure BST.Search(var item:PeopleKey ; var found:boolean);
      {will search the tree for item, and then put info in tree's item into
       the item to be sent back.  This is necessary in order to find the
       item's position in masterfile.}
      Procedure RecSearch(var root:PeopleKeyPointer ; var item:PeopleKey ;
         var found:boolean);
         begin
            if Root = nil then                   {empty tree}
               found:=false
            else
               begin                 { there is a non empty tree to search}
                  if item.ID < Root^.ID then           {descend before subtree}
                     recSearch(Root^.Before,item,found)
                  else if item.ID > Root^.ID then      {descend after subtree}
                     recSearch(Root^.after,item,found)
                  else                                 { ID found}
                     item:= Root^;
               end{else}
         end{recSearch};
      begin {.Search}
         found:= true;
         recSearch(root,item,found);
      end; {.Search}

   Procedure BST.Traverse; {for testing purposes only}
      Procedure recTraverse(root:PeopleKeyPointer);
         begin
             if root=nil then
                {do nothing, this is our base case}
             else
                begin
                   RecTraverse(root^.before);
                   write(root^.id:8,'  ');
                   writeln(root^.location);
                   RecTraverse(root^.after);
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

   Procedure BST.FirstItem(var item:PeopleKey);
      {will return the first item of the BST without changing its value}
      Procedure RecFirstItem(var item:PeopleKey ; root:peopleKeyPointer);
         begin
            if (root^.before=nil) then  {base case}
               Item:=root^
            else recFirstItem(item,root^.before);
         end{RecFirstItem};
      begin {FirstItem}
         if root=nil then
            writeln('Error:  trying to get an item from an empty tree')
         else
            RecFirstItem(item,root);
      end{FirstItem};

   Procedure BST.ShiftLocations(removed:integer);
      {After the record at file position Removed is deleted from the master
       file, every later record is one position earlier.}
      Procedure RecShift(root:PeopleKeyPointer);
         begin
            if root<>nil then
               begin
                  if root^.Location>removed then
                     dec(root^.Location);
                  RecShift(root^.before);
                  RecShift(root^.after);
               end; {if}
         end; {RecShift}
      begin {ShiftLocations}
         RecShift(root);
      end; {ShiftLocations}

begin
   {setup}
end.