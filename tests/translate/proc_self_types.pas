{ A procedural type whose routines use a global of that same type: a
  state machine (each state sets the next) and recursion through a
  global function variable. Rust's type aliases can't name themselves, so
  these types are structs; one is stored in a file too. Output from
  fpc -Mtp except the last two lines: TP's procedural value is 4 bytes
  (Free Pascal's 8, padded), and a stored one reads back as nil. }
program ProcSelfTypes;
type
  State = procedure;
  Func = function(N: Integer): LongInt;
var
  Current: State;
  Steps: Integer;
  Fact: Func;
  Saved: record
    Id: Integer;
    Next: State;
  end;
  F: file of State;

procedure Stop;
begin
  WriteLn('stop');
  Current := nil;
end;

procedure Second;
begin
  WriteLn('second');
  Current := Stop;
end;

procedure First;
begin
  WriteLn('first');
  Current := Second;
end;

function Factorial(N: Integer): LongInt;
begin
  if N <= 1 then
    Factorial := 1
  else
    Factorial := N * Fact(N - 1);
end;

begin
  Current := First;
  Steps := 0;
  while Assigned(Current) do
  begin
    Current;
    Inc(Steps);
  end;
  WriteLn(Steps);
  Fact := Factorial;
  WriteLn(Fact(10));
  Saved.Id := 7;
  Saved.Next := Second;
  WriteLn(SizeOf(Saved), ' ', Assigned(Saved.Next));
  Assign(F, 'proc_self_types.dat');
  Rewrite(F);
  Write(F, Saved.Next);
  Reset(F);
  Read(F, Current);
  Close(F);
  Erase(F);
  WriteLn(Assigned(Current));
end.
