{ Calls_Rust: a Turbo Pascal program that calls routines written in Rust,
  linked with $L as TP linked assembly (calls_rust.rs is the Rust half).
  The sort is in Rust, but asks a Pascal function which number comes
  first: Rust calling back into Pascal. See docs/calling_rust.md.

  Run it with: cargo run -p ruscal -- run examples/ruscal/calls_rust.pas }
program Calls_Rust;

{$L calls_rust.rs}

type
   TNumbers = array[1..8] of Integer;
   TLess = function(A, B: Integer): Boolean;

function EnvVar(const Name: string): string; external;
function WordCount(const Line: string): Integer; external;
function IsPrime(N: LongInt): Boolean; external;
procedure SortNumbers(var Numbers: TNumbers; Less: TLess); external;

{ Evens first, then odds, each smallest first. }
function EvensFirst(A, B: Integer): Boolean;
   begin
      if Odd(A) <> Odd(B) then
         EvensFirst := not Odd(A)
      else
         EvensFirst := A < B;
   end;

const
   Start: TNumbers = (31, 4, 15, 9, 26, 5, 3, 58);

var
   Numbers: TNumbers;
   Line: string;
   I: Integer;
   N: LongInt;

begin
   Line := EnvVar('USER');
   if Line = '' then
      Line := 'there';
   WriteLn('Hello, ', Line, '!');

   Line := 'the quick brown fox jumps over the lazy dog';
   WriteLn('"', Line, '" has ', WordCount(Line), ' words');

   Write('primes below 50:');
   for N := 1 to 50 do
      if IsPrime(N) then
         Write(' ', N);
   WriteLn;

   Numbers := Start;
   SortNumbers(Numbers, EvensFirst);
   Write('sorted, evens first:');
   for I := 1 to 8 do
      Write(' ', Numbers[I]);
   WriteLn;
end.
