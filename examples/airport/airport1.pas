{t                              Shannon Behrens and Mang Tran
                                Computer Programming II
                                3/19/97

Airport 1 - please see Airport4 for more info.  This unit establishes the
basic element type and maximum size of a queue.}

unit Airport1;

interface
   const
      maxsize = 100;
   type
      TimeType = 0..1440;                  {1440 minutes in a day}
      QRec    = record
         flightNum : string[10];
         JoinQ     : TimeType;
         LeaveQ    : TimeType;
      end; {record}
implementation

begin
   {none}
end.
