{ Calling Rust from Turbo Pascal: `external` routines are Rust functions
  in the file named by the $L directive before them (as TP linked .OBJ
  files), here rust_externals.rs (a DOS path, with a backslash), and a unit's in
  rustlink.rs. Output
  written by hand: Free Pascal can't link Rust. }
program RustExternals;
uses RustLink;
{$L .\rust_externals.rs}

procedure DoubleIt(var N: Integer); external;
function IsClose(A, b: Real): Boolean; external;
procedure Greet(const Name: string); external;

var
   N: Integer;
begin
   N := 21;
   DoubleIt(N);
   WriteLn(N);
   WriteLn(IsClose(0.1 + 0.2, 0.3), ' ', IsClose(1, 2));
   Greet('Pascal');
   WriteLn(Gcd(84, 36), ' ', Gcd(-12, 18));
   WriteLn(Reversed('stressed'));
end.
