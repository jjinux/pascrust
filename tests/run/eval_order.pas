program EvalOrder;
{ When a function in an expression has side effects: Write(F, G) writes
  F's value before G runs (TP defines it as Write(F); Write(G)), to the
  screen and to a file, and n := n + Bump reads n before Bump changes it,
  as Free Pascal does. A call that can't change n keeps n += ....
  The .out comes from fpc -Mtp, except that Free Pascal prints the line
  written to the '' file first: piped, its standard output is held until
  the program ends. TP (and PascRust) print in program order. }
var
  n, total: Integer;
  log: Text;
  line: string;

function Bump: Integer;
begin
  n := n + 10;
  Bump := 1;
end;

function F: Integer;
begin
  Write('F');
  F := 1;
end;

function G: Integer;
begin
  Write('G');
  G := 2;
end;

function Twice(x: Integer): Integer;
begin
  Twice := x * 2;
end;

function Mark: Integer;
begin
  Write(log, 'x');
  Mark := 2;
end;

begin
  n := 0;
  n := n + Bump;
  WriteLn('n + Bump: ', n);
  n := 0;
  n := Bump + n;
  WriteLn('Bump + n: ', n);
  WriteLn(F, G, ' Write(F, G)');
  Write(F, ' ');
  WriteLn(G);
  total := 1;
  total := total + Twice(3);
  WriteLn('total + Twice(3): ', total);
  Assign(log, 'order.txt');
  Rewrite(log);
  WriteLn(log, 'a', Mark, 'b');
  Close(log);
  Reset(log);
  ReadLn(log, line);
  Close(log);
  Erase(log);
  WriteLn('file: ', line);
  Assign(log, '');
  Rewrite(log);
  WriteLn(log, 'an empty file name is the screen');
  Close(log);
end.
