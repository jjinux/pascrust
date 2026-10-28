{t                              Shannon Behrens and Mang Tran
                                Computer Programming II
                                3/19/97

Airport 3 - a unit.  Please see Airport4 for more information.  This is
a "turbo charger" for the basic queue designed in airport2.  Please notice
that the calls to this unit must have the following syntax:
     MethodName (Q:Queue ; (if necessary) item:elementType);}

unit Airport3;

interface

   uses Airport1, Airport2;

   type
      QInteger = 0..maxsize;

   function Qsize(Q: Queue): QInteger;
   {Returns the size of the Queue}
   procedure DumpQ(Q: Queue);
   {Displays contents of the Queue from front to rear}
   procedure FrontOfQ(Q: Queue; var FrontRec: QRec);
   {Returns first record in queue without changing Queue}
   procedure RearOfQ(Q: Queue; var RearRec: QRec);
{Returns the last record in queue WITHOUT changing Queue}

implementation

   uses Crt;

   function Qsize(Q: Queue): QInteger;
      {Returns the size of the Queue}
      begin
         if (Q.front+1 mod maxsize) = Q.rear then
            Qsize := 0
         else if Q.front+1<Q.rear then
            Qsize := Q.rear - Q.front - 1
         else if (Q.front+1 mod maxsize) > Q.rear then
            Qsize := maxsize - (Q.front-Q.rear) - 1;
      end; {QSize}

   procedure DumpQ(Q: Queue);         {not var}
      {Displays contents of the Queue from front to rear}
      var
         item : QRec;                 {last item of Queue}
         a    : integer;
      procedure displayTime(Minutes: TimeType);
         begin
            write(Minutes div 60, ':', Minutes mod 60);
         end;                         {display time in military format}
      procedure Display(item: QRec);
         begin
            writeln;
            write(item.flightNum);
            gotoxy(13, wherey); {aligns everything up}
            displayTime(item.JoinQ);
            gotoxy(21, wherey);
            displayTime(item.LeaveQ);
         end; {Display - subProcedure of DumpQ}
      begin
         for a := 1 to Qsize(Q) do
         begin
            Q.removeQ(item);
            Display(item);             {a QRec}
         end;
      end; {DumpQ}

   procedure FrontOfQ(Q: Queue; var FrontRec: QRec);
      {Returns first record in queue without changing Queue}
      begin
         FrontRec := Q.QArray[Q.front];
      end; {FrontOfQ}

   procedure RearOfQ(Q: Queue; var RearRec: QRec);
      {Returns the last record in queue WITHOUT changing Queue}
      begin
         RearRec := Q.QArray[Q.rear]
      end; {RearOfQ}

begin {initialization}
   {none}
end.
