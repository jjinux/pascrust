program PrinterErrors;
{ Lst is the printer: always open, and only for writing. Input is only
  for reading. }
uses Printer;
var
  s: string;
begin
  ReadLn(Lst, s);
  Rewrite(Lst);
  WriteLn(Input, 'x');
end.
