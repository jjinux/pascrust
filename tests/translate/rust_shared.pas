{ Two units that link the same Rust file (shared.rs): it's one module.
  Output written by hand: Free Pascal can't link Rust. }
program RustShared;
uses LinkA, LinkB;
begin
   WriteLn(NameA, ' ', NameB);
end.
