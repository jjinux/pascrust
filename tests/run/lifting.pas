program Lifting;
{ Nested procedures and globals, which pascrust "lifts" into top-level
  Rust functions with extra parameters. }
var
  count: integer;
  log: string;

procedure Note(s: string);
begin
  log := log + s;
  count := count + 1;
end;

procedure Outer(var total: integer; step: integer);
var
  i: integer;

  procedure Middle;

    procedure Inner;
    begin
      { Two levels down: uses Outer's var parameter and local. }
      total := total + step * i;
    end;

  begin
    Inner;
    Note('m');
  end;

begin
  for i := 1 to 3 do
    Middle;
end;

procedure Hidden;
var
  count: integer;   { hides the global `count` ... }
begin
  count := 100;
  Note('h');        { ... but Note still updates the global one }
  writeln('local count ', count);
end;

var
  sum: integer;
begin
  count := 0;
  log := '';
  sum := 0;
  Outer(sum, 2);
  writeln('sum ', sum);
  Hidden;
  writeln('count ', count, ' log ', log);
end.
