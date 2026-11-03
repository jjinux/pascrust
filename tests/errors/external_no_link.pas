{ An external routine needs a $L directive before it naming its Rust file. }
program ExternalNoLink;
procedure Beep(Hz: Word); external;
begin
   Beep(440);
end.
