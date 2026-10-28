{t                              Shannon Behrens

Solves the following puzzle:

Fill a four by four grid with the correct permutation of these numbers:
     1 3 5 7 9 11 13 15 17 19 21 23 25 27 29 31
Each row of the grid should sum to 64.  The same must be true of each column
     and each diagnol.  Numbers may only be used once.}

uses Crt;

type
   GridType = array[1..16] of ShortInt;
   PossibilitiesType = array[1..16] of boolean;
var
   grid: GridType;                   {The grid specified in the comments}
   possibilities: PossibilitiesType; {Remembers what nums have been used}
   SpaceNum: byte;
   counter: LongInt;                 {Counts how many loops we've done}
   KeepGoing: boolean;

procedure Initialize(var possibilities: PossibilitiesType;
   var KeepGoing: boolean);
   {This resets all the possibilities to true.  This means they are available.}
   var a: byte;
   begin
      for a := 1 to 16 do
         possibilities[a] := true;
      KeepGoing := true;
   end; {Initialize}

procedure FillSpace(var SpaceNum: byte; var grid: GridType; var possibilities:
   PossibilitiesType);
   {Will fill one space with a valid number.  Then will make sure that that
    num is not used again via possibilities}
   var RandNum: byte;
   begin
      repeat
         RandNum := random(16)+1;
      until possibilities[RandNum] = true;    {number is available}
      possibilities[RandNum] := false;        {now it's unavailable}
      grid[SpaceNum] := (RandNum-1)*2 + 1;
   end; {FillRow}

procedure FillLastSpot(var SpaceNum: byte; var grid: GridType;
   var possibilities: PossibilitiesType; var KeepGoing: boolean);
   begin
      grid[SpaceNum] := 64 - (grid[SpaceNum-3]+grid[SpaceNum-2]+
         grid[SpaceNum-1]);
      if grid[SpaceNum]<1 then               {if first three sum to > 64}
         KeepGoing := false
      else if grid[SpaceNum]>31 then
         KeepGoing := false
      else if possibilities[(grid[SpaceNum]-1) div 2 + 1] = false then
         KeepGoing := false                  {if necessary number used}
      else possibilities[(grid[SpaceNum]-1) div 2 + 1] := false;
   end; {FillLastSpot}                       {got right num}

function RightGrid(var g: GridType): boolean;
   {Grid is not really variable, it's just faster this way}
   begin
      RightGrid :=
         (g[1]+g[2]+g[3]+g[4] = 64) and
         (g[5]+g[6]+g[7]+g[8] = 64) and
         (g[9]+g[10]+g[11]+g[12] = 64) and
         (g[13]+g[14]+g[15]+g[16] = 64) and
         (g[1]+g[5]+g[9]+g[13] = 64) and
         (g[2]+g[6]+g[10]+g[14] = 64) and
         (g[3]+g[7]+g[11]+g[15] = 64) and
         (g[4]+g[8]+g[12]+g[16] = 64) and
         (g[1]+g[6]+g[11]+g[16] = 64) and
         (g[4]+g[7]+g[10]+g[13] = 64);
   end; {RightGrid}

procedure ShowGrid(grid: GridType);
   var a, r, c: byte;
   begin
      writeln;
      writeln('Entered ShowGrid!!!');
      a := 0;
      for r := 1 to 4 do
      begin
         for c := 1 to 4 do
         begin
            inc(a);
            write(grid[a], ' ');
         end; {for}
         writeln;
      end; {for}
   end; {ShowGrid}

procedure SimpleInit(var counter: LongInt);
   begin
      randomize;
      clrscr;
      textcolor(yellow);
      textbackground(blue);
      counter := 0;
   end; {SimpleInit}

begin
   repeat
      inc(counter);
      Initialize(possibilities, KeepGoing);
      for SpaceNum := 1 to 3 do
         FillSpace(SpaceNum, grid, possibilities);
      inc(SpaceNum);
      FillLastSpot(SpaceNum, grid, possibilities, KeepGoing);
      if KeepGoing then
      begin
         for SpaceNum := 5 to 7 do
            FillSpace(SpaceNum, grid, possibilities);
         inc(SpaceNum);
         FillLastSpot(SpaceNum, grid, possibilities, KeepGoing);
         if KeepGoing then
         begin
            for SpaceNum := 9 to 11 do
               FillSpace(SpaceNum, grid, possibilities);
            inc(SpaceNum);
            FillLastSpot(SpaceNum, grid, possibilities, KeepGoing);
            if KeepGoing then
            begin
               for SpaceNum := 13 to 15 do
                  FillSpace(SpaceNum, grid, possibilities);
               inc(SpaceNum);
               FillLastSpot(SpaceNum, grid, possibilities, KeepGoing);
            end; {if}
         end; {if}
      end; {if}
      if counter mod 10000 = 0
         then write(counter, '   ');
   until KeepGoing and (RightGrid(grid) = true);
   ShowGrid(grid);
end.
