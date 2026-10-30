program CharArrayIOErrors;
{ Only a zero-based Char array is a C-style string to Str, Read and Val. }
var
  name: array[1..10] of Char;
  n, code: Integer;
begin
  Str(42, name);
  ReadLn(name);
  Val(name, n, code);
end.
