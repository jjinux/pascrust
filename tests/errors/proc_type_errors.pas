program ProcTypeErrors;
{ What Turbo Pascal refuses as a procedural value: a nested routine, a
  method, a standard routine, one with another signature; a function
  returning a procedural type; comparing procedural values without `@`;
  and a routine's name as a `var` argument. }
type
   Func = function(X: Real): Real;
   Maker = function: Func;
   TObj = object
      function Half(X: Real): Real;
   end;
var
   F, G: Func;
   O: TObj;

function TObj.Half(X: Real): Real;
   begin
      Half := X / 2;
   end;

function Twice(X: Integer): Integer;
   begin
      Twice := 2 * X;
   end;

procedure Reset(var H: Func);
   begin
      H := nil;
   end;

procedure Outer;

   function Inner(X: Real): Real;
      begin
         Inner := X;
      end;

   begin
      F := Inner;
   end;

begin
   F := O.Half;
   F := Sin;
   F := Twice;
   if F = G then
      WriteLn('same');
   Reset(Twice);
end.
