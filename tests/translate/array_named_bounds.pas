{ An array's bounds keep the constants they name in the Rust (its size
  is N as usize there), so changing N in the Rust resizes it. }
program ArrayNamedBounds;
const
   N = 10;
   Lo = 3;
   Hi = Lo + 4;
   Big = 40000;
type
   Index = 1..N;
   TArr = array[1..N] of Integer;
   Zero = array[0..N - 1] of Byte;
   Mid = array[Lo..Hi] of Char;
   ByIndex = array[Index] of Integer;
   Grid = array[1..N, 0..N] of Byte;
   Wide = array[Big..Big + 2] of Byte;
   Plain = array[1..5] of Integer;
var
   a: TArr;
   z: Zero;
   m: Mid;
   b: ByIndex;
   g: Grid;
   w: Wide;
   p: Plain;
   v: array[2..N * 2] of Integer;
begin
   a[N] := 1;
   z[N - 1] := 2;
   m[Hi] := 'x';
   b[N] := 3;
   g[N, N] := 4;
   w[Big + 2] := 5;
   v[N * 2] := 6;
   p[5] := 7;
   WriteLn(a[N], z[N - 1], m[Hi], b[N], g[N, N], w[Big + 2], v[N * 2], p[5], ' ', High(v), ' ', SizeOf(g));
end.
