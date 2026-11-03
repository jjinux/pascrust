{ A unit whose routines are written in Rust (rustlink.rs), linked with
  $L as TP linked an .OBJ file. Used by rust_externals.pas. }
unit RustLink;

interface

   function Gcd(A, B: LongInt): LongInt;
   function Reversed(const S: string): string;

implementation

   {$L rustlink.rs}
   function Gcd(A, B: LongInt): LongInt; external;
   function Reversed(const S: string): string; external;

end.
