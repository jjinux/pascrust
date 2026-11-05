{ A unit with variables, built with `pascrust build --lib` and used from
  tests/embed/counter_host.rs: its variables (and Helper's) are fields of
  the generated CounterContext. }
unit Counter;
interface
   uses Helper;
   type
      TBumper = procedure(N: Integer);
      TTally = object
         Total: Integer;
         procedure Add(N: Integer);
      end;
   var
      Count: Integer;
      Hook: TBumper;
      Log: string;
   procedure Bump(N: Integer);
   function Current: Integer;
   procedure Swap(var A, B: Integer);
   procedure New(N: Integer);
   function Pure(X: Integer): Integer;
   procedure CallHook(N: Integer);
   procedure TickTwice;
   procedure AddTimes(N, Times: Integer);
   function Probe: Integer;
   function Next(N: Integer): Integer;
implementation
   const
      Secret: Integer = 7;
   var
      Internal: Integer;
   procedure Bump(N: Integer);
      begin
         Count := Count + N + Internal;
         HelperCount := HelperCount + 1;
      end;
   function Current: Integer;
      begin
         Current := Count;
      end;
   procedure Swap(var A, B: Integer);
      var T: Integer;
      begin
         T := A; A := B; B := T;
         Log := Log + 's';
      end;
   procedure New(N: Integer);
      begin
         Count := N;
      end;
   function Pure(X: Integer): Integer;
      begin
         Pure := X * 2;
      end;
   procedure TTally.Add(N: Integer);
      begin
         Total := Total + N;
         Count := Count + N;
      end;
   procedure CallHook(N: Integer);
      begin
         Hook(N);
      end;
   procedure TickTwice;
      begin
         Tick;
         Tick;
      end;
   { A value parameter the routine changes (`Times`). }
   procedure AddTimes(N, Times: Integer);
      begin
         while Times > 0 do
         begin
            Count := Count + N;
            Times := Times - 1;
         end;
      end;
   { An I/O error under $I-, handled by the Pascal code itself. }
   function Probe: Integer;
      var
         F: Text;
      begin
         Assign(F, 'no_such_file.txt');
         {$I-}
         Reset(F);
         {$I+}
         Probe := IOResult;
      end;
   { Integer arithmetic wraps around, as in TP: Next(32767) is -32768. }
   function Next(N: Integer): Integer;
      begin
         Next := N + 1;
      end;
begin
   Internal := Secret;
   Hook := Bump;
   HelperCount := 100;
   Log := '';
end.
