program EnumPastEnd;
{ Succ and Pred of an enum constant must stay in the enum, and a program's
  own function named like a builtin isn't a constant. }
type
  Day = (Mon, Tue, Wed);
const
  After = Succ(Wed);
  Before = Pred(Mon);

function Succ(n: Integer): Integer;
begin
  Succ := n * 10;
end;

const
  NotConstant = Succ(2);
begin
end.
