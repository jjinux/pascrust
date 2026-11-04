{ A unit whose routine is in shared.rs, which LinkA and LinkB both link.
  Used by rust_shared.pas. }
unit LinkA;

interface

   function NameA: Integer;

implementation

   {$L shared.rs}
   function NameA: Integer; external;

end.
