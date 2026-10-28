program SystemVarErrors;
{ Runtime variables are read or set with :=, and Input and Output are
  always the console. }
begin
  Inc(RandSeed);
  Assign(Output, 'log.txt');
  Reset(Input);
end.
