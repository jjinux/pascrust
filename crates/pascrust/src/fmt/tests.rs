use super::*;

fn fmt(source: &str) -> String {
    let out = format_files(&[source.as_bytes()]).map_err(|(_, d)| d.message).unwrap();
    String::from_utf8(out.into_iter().next().unwrap()).unwrap()
}

/// Formatting must be idempotent, so every test also formats its output
/// again.
fn check(source: &str, expected: &str) {
    let once = fmt(source);
    assert_eq!(once, expected);
    assert_eq!(fmt(&once), expected, "formatting twice changed the result");
}

#[test]
fn nested_routines_and_statements() {
    check(
        "PROGRAM Demo;
VAR x : Integer;
Procedure Outer(n:integer);
var y:integer;
procedure Inner;
begin
writeln(n)
end;
begin
if n>0 then
y:=n
else
begin
y:=-n;
Inner
end;
while y>0 do y:=y-1;
end;
BEGIN
Outer(3)
END.
",
        "program Demo;
var x: integer;
procedure Outer(n: integer);
   var y: integer;
   procedure Inner;
      begin
         writeln(n)
      end;
   begin
      if n>0 then
         y := n
      else
      begin
         y := -n;
         Inner
      end;
      while y>0 do y := y-1;
   end;
begin
   Outer(3)
end.
",
    );
}

#[test]
fn case_repeat_and_comments() {
    check(
        "program P;
var c: char;
begin
  repeat
  read(c);
      { which key? }
  case c of
  'a'  : writeln('A');
  'b'  : writeln('B');   { the other one }
  else writeln('?')
  end
  until c = 'q';
end.
",
        "program P;
var c: char;
begin
   repeat
      read(c);
      { which key? }
      case c of
         'a' : writeln('A');
         'b' : writeln('B');    { the other one }
         else writeln('?')
      end
   until c = 'q';
end.
",
    );
}

#[test]
fn units_records_and_objects() {
    check(
        "unit Shapes;
interface
type
Point = record
x, y: integer;
end;
Shape = object
origin: Point;
procedure Move(dx, dy: integer);
end;
implementation
procedure Shape.Move(dx, dy: integer);
begin
origin.x := origin.x + dx;
end;
end.
",
        "unit Shapes;
interface
   type
      Point = record
         x, y: integer;
      end;
      Shape = object
         origin: Point;
         procedure Move(dx, dy: integer);
      end;
implementation
   procedure Shape.Move(dx, dy: integer);
      begin
         origin.x := origin.x + dx;
      end;
end.
",
    );
}

#[test]
fn spacing_keeps_strings_comments_and_formats() {
    check(
        "program P;
const Msg='a := b ,c';
var r : real ;
begin
r:=1.5 ;   {  odd   spacing kept }
writeln( Msg , r:8:2 );
end.
",
        "program P;
const Msg = 'a := b ,c';
var r: real;
begin
   r := 1.5;  {  odd   spacing kept }
   writeln(Msg, r:8:2);
end.
",
    );
}

#[test]
fn aligned_declarations_stay_aligned() {
    check(
        "program P;
var
   name     : string;   {who}
   a,b      : integer;  {two numbers}
   i        : integer;
begin
end.
",
        "program P;
var
   name : string;       {who}
   a, b : integer;      {two numbers}
   i    : integer;
begin
end.
",
    );
}

#[test]
fn identifiers_take_their_most_common_spelling() {
    check(
        "program P;
var Total: integer;
begin
  total := 1;
  TOTAL := Total + total;
  WriteLn(total)
end.
",
        "program P;
var total: integer;
begin
   total := 1;
   total := total + total;
   WriteLn(total)
end.
",
    );
}

#[test]
fn multi_line_comments_move_with_their_first_line() {
    check(
        "program P;
      {A comment
       over three
       lines}
begin
end.
",
        "program P;
{A comment
 over three
 lines}
begin
end.
",
    );
}

#[test]
fn dos_line_endings() {
    let unix = "program P;\nbegin\n   writeln('hi');\nend.\n";
    assert_eq!(fmt(&unix.replace('\n', "\r\n")), unix);
}

#[test]
fn files_share_spellings() {
    let main = b"program Main; uses Lib; begin DoIt end.\n" as &[u8];
    let lib = b"unit Lib; interface procedure DOIT; implementation procedure doit; begin end; end.\n" as &[u8];
    let out = format_files(&[main, lib]).map_err(|(_, d)| d.message).unwrap();
    // Three spellings, one each: the first seen wins.
    assert!(String::from_utf8_lossy(&out[1]).contains("procedure DoIt;"));
}
