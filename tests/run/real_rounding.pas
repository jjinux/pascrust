program RealRounding;
{ Turbo Pascal rounds halves away from zero when it prints a real. }
begin
   writeln(0.125:0:2, ' ', 2.5:0:0, ' ', 0.5:0:0, ' ', 2.675:0:2, ' ', 2.125:8:2);
   writeln(9.995:0:2, ' ', 99.5:0:0, ' ', -0.004:0:2, ' ', -0.006:0:2, ' ', 0.0:0:3);
   writeln(1234567.891:0:1, ' ', 1e10:0:0, ' ', 0.000123:0:5, ' ', -3.14159:10:3);
   writeln(123.456:8:0, ' ', 0.05:0:1, ' ', 0.15:0:1, ' ', 1.45:0:1);
end.
