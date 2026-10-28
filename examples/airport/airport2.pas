{t                              Shannon Behrens and Mang Tran
                                Computer Programming II
                                3/19/97

Airport2 - a unit.  Please see Airport4 for more information.  This unit
defines the basic Qtype as well as its general operations.  It is programmed
in object oriented code}

unit Airport2;

interface

   uses Airport1;                   {for Qrec declarations}

   type
      Queue = object
         QArray : array[0..maxsize] of QRec;
         front  : 0 .. maxsize; {first member of queue}
         rear   : 0 .. maxsize; {last member in queue}
         procedure createQ;
         procedure addQ(item: QRec);
         procedure removeQ(var item: QRec);
         function fullQ: boolean;
      end; {object}

implementation

   procedure Queue.createQ;              {makes an empy queue}
      begin
         front := 0;                     {a nonempty Q has information}
         rear := 1;                      {only BETWEEN front and rear}
      end; {procedure Queue.createQ}

   procedure Queue.addQ(item: QRec);
      var
         newrear: 0 .. maxsize;
      begin
         newrear := rear+1 mod maxsize;
         if rear = front then           {empty queue}
         begin
            QArray[front+1 mod maxsize] := item;
            rear := rear+2 mod maxsize
         end
         else if newrear <> front then  {not full queue}
         begin
            QArray[newrear-1] := item;
            rear := newrear;
         end
         else {newrear=front}           {full queue}
            writeln('Queue error: full queue');
      end; {procedure Queue.addQ}

   procedure Queue.removeQ(var item: QRec);
      begin
         if front <> (rear+1 mod maxsize) then
         begin
            item := QArray[front+1];
            front := front+1 mod maxsize;
         end
         else
            writeln('Queue error: queue already empty');
      end; {procedure.removeQ}

   function Queue.fullQ: boolean;
      begin
         fullQ := (rear+1 mod maxsize) = front;
      end; {fullQ}

begin {initialization}
   {none}
end.
