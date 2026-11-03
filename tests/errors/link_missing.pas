{ The Rust file a $L directive names must exist. }
program LinkMissing;
{$L no_such_file.rs}
procedure Beep(Hz: Word); external;
begin
   Beep(440);
end.
