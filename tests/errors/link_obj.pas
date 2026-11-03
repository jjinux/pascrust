{ An object file can't be linked: it's code for DOS. }
program LinkObj;
{$L sound.obj}
procedure Beep(Hz: Word); external;
begin
   Beep(440);
end.
