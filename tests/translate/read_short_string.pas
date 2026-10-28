program ReadShortString;
{ Read into a string[n] takes at most n characters; the rest of the line is
  left for the next Read. }
type
   Rec = record
      code: string[2];
   end;
var
   s: string[5];
   t: string;
   r: Rec;
begin
   read(s);
   readln(t);
   writeln(s, '|', t, '|', length(s));
   readln(r.code);
   writeln(r.code);
end.
