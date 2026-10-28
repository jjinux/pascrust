program Files;
{ Text files and typed files (file of record) with random access. }
type
  Person = record
    name: string[20];
    age: integer;
  end;
var
  f: file of Person;
  t: Text;
  p: Person;
  i: integer;
  line: string;
  n: integer;
begin
  assign(f, 'people.dat');
  rewrite(f);
  p.name := 'Ann';   p.age := 31; write(f, p);
  p.name := 'Bob';   p.age := 42; write(f, p);
  p.name := 'Carol'; p.age := 27; write(f, p);
  writeln('size ', filesize(f), ' pos ', filepos(f));
  seek(f, 1);
  read(f, p);
  writeln('record 1: ', p.name, ' ', p.age, ', now at ', filepos(f));
  seek(f, 0);
  while not eof(f) do
  begin
    read(f, p);
    write(p.name, ' ');
  end;
  writeln;
  { Change a record in place. }
  seek(f, 2);
  p.name := 'Dave';
  p.age := 50;
  write(f, p);
  seek(f, 2);
  read(f, p);
  writeln('record 2 is now ', p.name);
  close(f);
  erase(f);

  assign(t, 'notes.txt');
  rewrite(t);
  writeln(t, 'first line');
  writeln(t, 42, ' ', 3.5:0:1);
  close(t);
  append(t);
  writeln(t, 'appended');
  close(t);
  reset(t);
  i := 0;
  while not eof(t) do
  begin
    readln(t, line);
    i := i + 1;
    writeln(i, ': ', line);
  end;
  close(t);
  reset(t);
  readln(t);
  read(t, n);
  writeln('number on line 2: ', n);
  close(t);
  erase(t);
end.
