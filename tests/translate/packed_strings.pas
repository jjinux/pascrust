program PackedStrings;
{ Packed strings, array[M..N] of Char: a string constant assigned to one
  (exactly N characters, or fewer for a zero-based array, padded with #0),
  and one used as a string (assigned, compared, joined with +, passed as a
  string parameter; a zero-based one up to its #0), and a typed constant.
  Output from fpc -Mtp. }
type
   TName = array[1..5] of Char;
const
   Hello: TName = 'Hello';
var
   name, other: TName;
   z: array[0..7] of Char;
   s: string;
procedure Show(s: string);
   begin
      WriteLn('<', s, '>');
   end;

begin
   name := 'Smith';
   other := name;
   s := name;
   WriteLn(s, ' ', Length(s));
   WriteLn(name = 'Smith', ' ', name = other, ' ', name < 'Smiti', ' ', 'x' + name + 'y');
   s := 'Smith';
   WriteLn(name = s);
   z := 'ab';
   s := z;
   WriteLn(Length(s), ' ', Ord(z[2]), ' ', Ord(z[7]));
   z := 'abcdefgh';
   s := z;
   WriteLn(s, ' ', Length(s));
   Show(Hello + '!');
   name := Hello;
   Show(name);
end.
