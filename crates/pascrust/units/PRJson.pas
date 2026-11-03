{ PRJson: read and write JSON. Part of PascRust (MIT); written in Turbo
  Pascal, so it also compiles in TP 7 itself.

  A JSON value is a record on the heap, reached through a PJsonValue; an
  array's or object's members are a linked list of them. JsonLoad and
  JsonParse read JSON (nil, with the reason in JsonError, if it isn't);
  the Json... functions look at values and build them; JsonWrite and
  JsonSave write them out; JsonFree frees a value and all it holds.

  Strings are Turbo Pascal strings, so a JSON string (or key) of more
  than 255 characters is cut to 255. A \u escape above 255 reads as '?'.
  Numbers are Reals. Array items count from 0, as TCollection's do. }
unit PRJson;

interface

   type
      TJsonKind = (jkNull, jkBoolean, jkNumber, jkString, jkArray, jkObject);
      PJsonValue = ^TJsonValue;
      TJsonValue = record
         Kind: TJsonKind;
         Bool: Boolean;           { a Boolean's value }
         Number: Real;            { a number's value }
         Str: string;             { a string's value }
         Key: string;             { its name, as a member of an object }
         Count: Integer;          { an array's or object's number of members }
         First, Last: PJsonValue; { an array's or object's members }
         Next: PJsonValue;        { the next member of the array or object it's in }
      end;

   var
      JsonError: string;          { why the last JsonLoad or JsonParse failed, or '' }

   function JsonParse(const S: string): PJsonValue;
   function JsonLoad(const FileName: string): PJsonValue;

   function JsonKind(V: PJsonValue): TJsonKind;
   function JsonCount(V: PJsonValue): Integer;
   function JsonItem(V: PJsonValue; Index: Integer): PJsonValue;
   function JsonGet(V: PJsonValue; const Key: string): PJsonValue;
   function JsonString(V: PJsonValue): string;
   function JsonNumber(V: PJsonValue): Real;
   function JsonInteger(V: PJsonValue): LongInt;
   function JsonBool(V: PJsonValue): Boolean;
   function JsonGetString(V: PJsonValue; const Key: string): string;
   function JsonGetNumber(V: PJsonValue; const Key: string): Real;
   function JsonGetInteger(V: PJsonValue; const Key: string): LongInt;
   function JsonGetBool(V: PJsonValue; const Key: string): Boolean;

   function JsonNewNull: PJsonValue;
   function JsonNewBool(B: Boolean): PJsonValue;
   function JsonNewNumber(X: Real): PJsonValue;
   function JsonNewString(const S: string): PJsonValue;
   function JsonNewArray: PJsonValue;
   function JsonNewObject: PJsonValue;
   procedure JsonAdd(Arr, V: PJsonValue);
   procedure JsonSet(Obj: PJsonValue; const Key: string; V: PJsonValue);

   procedure JsonWrite(var F: Text; V: PJsonValue; Indent: Integer);
   procedure JsonSave(const FileName: string; V: PJsonValue; Indent: Integer);
   function JsonToString(V: PJsonValue): string;

   procedure JsonFree(V: PJsonValue);

implementation

   { ----- Building values ----- }

   function NewValue(Kind: TJsonKind): PJsonValue;
      var
         V: PJsonValue;
      begin
         New(V);
         V^.Kind := Kind;
         V^.Bool := False;
         V^.Number := 0;
         V^.Str := '';
         V^.Key := '';
         V^.Count := 0;
         V^.First := nil;
         V^.Last := nil;
         V^.Next := nil;
         NewValue := V;
      end;

   function JsonNewNull: PJsonValue;
      begin
         JsonNewNull := NewValue(jkNull);
      end;

   function JsonNewBool(B: Boolean): PJsonValue;
      var
         V: PJsonValue;
      begin
         V := NewValue(jkBoolean);
         V^.Bool := B;
         JsonNewBool := V;
      end;

   function JsonNewNumber(X: Real): PJsonValue;
      var
         V: PJsonValue;
      begin
         V := NewValue(jkNumber);
         V^.Number := X;
         JsonNewNumber := V;
      end;

   function JsonNewString(const S: string): PJsonValue;
      var
         V: PJsonValue;
      begin
         V := NewValue(jkString);
         V^.Str := S;
         JsonNewString := V;
      end;

   function JsonNewArray: PJsonValue;
      begin
         JsonNewArray := NewValue(jkArray);
      end;

   function JsonNewObject: PJsonValue;
      begin
         JsonNewObject := NewValue(jkObject);
      end;

   { Put V at the end of a list of members. }
   procedure Append(List, V: PJsonValue);
      begin
         if List^.Last = nil then
            List^.First := V
         else
            List^.Last^.Next := V;
         List^.Last := V;
         Inc(List^.Count);
      end;

   procedure JsonAdd(Arr, V: PJsonValue);
      begin
         if (Arr <> nil) and (Arr^.Kind = jkArray) and (V <> nil) then
            Append(Arr, V);
      end;

   procedure JsonFree(V: PJsonValue);
      var
         Member, Following: PJsonValue;
      begin
         if V = nil then
            Exit;
         Member := V^.First;
         while Member <> nil do
         begin
            Following := Member^.Next;
            JsonFree(Member);
            Member := Following;
         end;
         Dispose(V);
      end;

   { Set Obj's member Key to V, in place of one it had. }
   procedure JsonSet(Obj: PJsonValue; const Key: string; V: PJsonValue);
      var
         Member, Before: PJsonValue;
      begin
         if (Obj = nil) or (Obj^.Kind <> jkObject) or (V = nil) then
            Exit;
         V^.Key := Key;
         Before := nil;
         Member := Obj^.First;
         while (Member <> nil) and (Member^.Key <> Key) do
         begin
            Before := Member;
            Member := Member^.Next;
         end;
         if Member = nil then
         begin
            Append(Obj, V);
            Exit;
         end;
         { Replace Member with V where it is. }
         V^.Next := Member^.Next;
         if Before = nil then
            Obj^.First := V
         else
            Before^.Next := V;
         if Obj^.Last = Member then
            Obj^.Last := V;
         Member^.Next := nil;
         JsonFree(Member);
      end;

   { ----- Looking at values ----- }

   function JsonKind(V: PJsonValue): TJsonKind;
      begin
         if V = nil then
            JsonKind := jkNull
         else
            JsonKind := V^.Kind;
      end;

   function JsonCount(V: PJsonValue): Integer;
      begin
         if V = nil then
            JsonCount := 0
         else
            JsonCount := V^.Count;
      end;

   function JsonItem(V: PJsonValue; Index: Integer): PJsonValue;
      var
         Member: PJsonValue;
      begin
         Member := nil;
         if (V <> nil) and (Index >= 0) then
         begin
            Member := V^.First;
            while (Member <> nil) and (Index > 0) do
            begin
               Member := Member^.Next;
               Dec(Index);
            end;
         end;
         JsonItem := Member;
      end;

   function JsonGet(V: PJsonValue; const Key: string): PJsonValue;
      var
         Member: PJsonValue;
      begin
         Member := nil;
         if (V <> nil) and (V^.Kind = jkObject) then
         begin
            Member := V^.First;
            while (Member <> nil) and (Member^.Key <> Key) do
               Member := Member^.Next;
         end;
         JsonGet := Member;
      end;

   function JsonString(V: PJsonValue): string;
      begin
         if (V <> nil) and (V^.Kind = jkString) then
            JsonString := V^.Str
         else
            JsonString := '';
      end;

   function JsonNumber(V: PJsonValue): Real;
      begin
         if (V <> nil) and (V^.Kind = jkNumber) then
            JsonNumber := V^.Number
         else
            JsonNumber := 0;
      end;

   function JsonInteger(V: PJsonValue): LongInt;
      begin
         JsonInteger := Round(JsonNumber(V));
      end;

   function JsonBool(V: PJsonValue): Boolean;
      begin
         JsonBool := (V <> nil) and (V^.Kind = jkBoolean) and V^.Bool;
      end;

   function JsonGetString(V: PJsonValue; const Key: string): string;
      begin
         JsonGetString := JsonString(JsonGet(V, Key));
      end;

   function JsonGetNumber(V: PJsonValue; const Key: string): Real;
      begin
         JsonGetNumber := JsonNumber(JsonGet(V, Key));
      end;

   function JsonGetInteger(V: PJsonValue; const Key: string): LongInt;
      begin
         JsonGetInteger := JsonInteger(JsonGet(V, Key));
      end;

   function JsonGetBool(V: PJsonValue; const Key: string): Boolean;
      begin
         JsonGetBool := JsonBool(JsonGet(V, Key));
      end;

   { ----- Reading JSON ----- }

   { The reader works through a string or a file one character at a time:
     Ch is the character it's at (#0 at the end), Where how many it has
     read, for messages. }
   var
      FromFile: Boolean;
      Source: string;
      SourcePos: Integer;
      Input2: Text;
      Ch: Char;
      AtEnd: Boolean;
      Where: LongInt;
      Failed: Boolean;

   procedure NextChar;
      begin
         AtEnd := False;
         if FromFile then
         begin
            if Eof(Input2) then
               AtEnd := True
            else
               Read(Input2, Ch);
         end
         else if SourcePos > Length(Source) then
            AtEnd := True
         else
         begin
            Ch := Source[SourcePos];
            Inc(SourcePos);
         end;
         if AtEnd then
            Ch := #0
         else
            Inc(Where);
      end;

   procedure Fail(const Why: string);
      var
         At: string;
      begin
         if not Failed then
         begin
            Failed := True;
            Str(Where, At);
            if AtEnd then
               JsonError := Why + ' at the end'
            else
               JsonError := Why + ' at character ' + At;
         end;
      end;

   procedure SkipSpace;
      begin
         while not AtEnd and (Ch in [' ', #9, #10, #13]) do
            NextChar;
      end;

   procedure Expect(C: Char);
      begin
         SkipSpace;
         if Ch = C then
            NextChar
         else
            Fail('expected ''' + C + '''');
      end;

   { The value of four hex digits after \u. }
   function HexDigits: Integer;
      var
         I, N, D: Integer;
      begin
         N := 0;
         for I := 1 to 4 do
         begin
            NextChar;
            case UpCase(Ch) of
               '0'..'9': D := Ord(Ch) - Ord('0');
               'A'..'F': D := Ord(UpCase(Ch)) - Ord('A') + 10;
               else
                  begin
                     Fail('expected a hex digit');
                     D := 0;
                  end;
            end;
            N := N * 16 + D;
         end;
         HexDigits := N;
      end;

   { A string, with Ch at its opening quote. }
   function ParseString: string;
      var
         S: string;
         C: Char;
         Code: Integer;
      begin
         S := '';
         NextChar;
         while not Failed and (Ch <> '"') do
         begin
            if AtEnd then
               Fail('a string doesn''t end')
            else
            begin
               C := Ch;
               if C = '\' then
               begin
                  NextChar;
                  case Ch of
                     '"', '\', '/': C := Ch;
                     'b': C := #8;
                     'f': C := #12;
                     'n': C := #10;
                     'r': C := #13;
                     't': C := #9;
                     'u':
                     begin
                        Code := HexDigits;
                        if Code <= 255 then
                           C := Chr(Code)
                        else
                           C := '?';
                     end;
                     else
                        Fail('unknown escape \' + Ch);
                  end;
               end;
               if Length(S) < 255 then
                  S := S + C;
               NextChar;
            end;
         end;
         NextChar;
         ParseString := S;
      end;

   { A number, with Ch at its first character. }
   function ParseNumber: PJsonValue;
      var
         S: string;
         X: Real;
         Code: Integer;
      begin
         S := '';
         while not AtEnd and (Ch in ['0'..'9', '-', '+', '.', 'e', 'E']) do
         begin
            S := S + Ch;
            NextChar;
         end;
         Val(S, X, Code);
         if Code <> 0 then
            Fail('a bad number');
         ParseNumber := JsonNewNumber(X);
      end;

   { true, false or null, with Ch at its first letter. }
   function ParseWord: PJsonValue;
      var
         W: string;
      begin
         W := '';
         while not AtEnd and (Ch in ['a'..'z']) do
         begin
            W := W + Ch;
            NextChar;
         end;
         if W = 'true' then
            ParseWord := JsonNewBool(True)
         else if W = 'false' then
            ParseWord := JsonNewBool(False)
         else
         begin
            if W <> 'null' then
               Fail('expected a value');
            ParseWord := JsonNewNull;
         end;
      end;

   function ParseValue: PJsonValue; forward;

   { An array or object, with Ch at its opening bracket. }
   function ParseList(Kind: TJsonKind): PJsonValue;
      var
         List, Member: PJsonValue;
         Key: string;
         Close: Char;
         Done: Boolean;
      begin
         List := NewValue(Kind);
         if Kind = jkArray then
            Close := ']'
         else
            Close := '}';
         NextChar;
         SkipSpace;
         Done := Ch = Close;
         if Done then
            NextChar;
         while not Done and not Failed do
         begin
            SkipSpace;
            Key := '';
            if Kind = jkObject then
            begin
               if Ch = '"' then
                  Key := ParseString
               else
                  Fail('expected a key');
               Expect(':');
            end;
            if not Failed then
            begin
               Member := ParseValue;
               Member^.Key := Key;
               Append(List, Member);
               SkipSpace;
               if Ch = ',' then
                  NextChar
               else
               begin
                  Expect(Close);
                  Done := True;
               end;
            end;
         end;
         ParseList := List;
      end;

   function ParseValue: PJsonValue;
      begin
         SkipSpace;
         case Ch of
            '{': ParseValue := ParseList(jkObject);
            '[': ParseValue := ParseList(jkArray);
            '"': ParseValue := JsonNewString(ParseString);
            '-', '0'..'9': ParseValue := ParseNumber;
            'a'..'z': ParseValue := ParseWord;
            else
               begin
                  Fail('expected a value');
                  ParseValue := JsonNewNull;
               end;
         end;
      end;

   { The whole text is one value, with nothing after it. }
   function ParseAll: PJsonValue;
      var
         V: PJsonValue;
      begin
         JsonError := '';
         Failed := False;
         Where := 0;
         NextChar;
         V := ParseValue;
         SkipSpace;
         if not AtEnd then
            Fail('more after the value');
         if Failed then
         begin
            JsonFree(V);
            V := nil;
         end;
         ParseAll := V;
      end;

   function JsonParse(const S: string): PJsonValue;
      begin
         FromFile := False;
         Source := S;
         SourcePos := 1;
         JsonParse := ParseAll;
      end;

   function JsonLoad(const FileName: string): PJsonValue;
      begin
         Assign(Input2, FileName);
         {$I-}
         Reset(Input2);
         {$I+}
         if IOResult <> 0 then
         begin
            JsonError := 'cannot open ' + FileName;
            JsonLoad := nil;
            Exit;
         end;
         FromFile := True;
         JsonLoad := ParseAll;
         Close(Input2);
      end;

   { ----- Writing JSON ----- }

   { A number as JSON writes it: a whole number without a point, others
     without trailing zeros. }
   function NumberText(X: Real): string;
      var
         S: string;
      begin
         if (Frac(X) = 0) and (Abs(X) < 2147483647.0) then
            Str(Trunc(X), S)
         else
         begin
            Str(X:0:9, S);
            while S[Length(S)] = '0' do
               Delete(S, Length(S), 1);
            if S[Length(S)] = '.' then
               Delete(S, Length(S), 1);
         end;
         NumberText := S;
      end;

   { A string in quotes, with the characters JSON can't have as they are
     escaped. }
   function Quoted(const S: string): string;
      const
         Hex: string[16] = '0123456789abcdef';
      var
         Q: string;
         I: Integer;
      begin
         Q := '"';
         for I := 1 to Length(S) do
            case S[I] of
               '"': Q := Q + '\"';
               '\': Q := Q + '\\';
               #8: Q := Q + '\b';
               #9: Q := Q + '\t';
               #10: Q := Q + '\n';
               #12: Q := Q + '\f';
               #13: Q := Q + '\r';
               #0..#7, #11, #14..#31: Q := Q + '\u00' + Hex[Ord(S[I]) div 16 + 1] + Hex[Ord(S[I]) mod 16 + 1];
               else
                  Q := Q + S[I];
            end;
         Quoted := Q + '"';
      end;

   { The writer goes to a file, or to a string for JsonToString. }
   var
      ToString: Boolean;
      Output2: string;

   procedure Put(var F: Text; const S: string);
      begin
         if ToString then
         begin
            if Length(Output2) + Length(S) <= 255 then
               Output2 := Output2 + S
            else
               Output2 := Copy(Output2 + S, 1, 255);
         end
         else
            Write(F, S);
      end;

   procedure NewLine(var F: Text; Indent, Depth: Integer);
      var
         I: Integer;
      begin
         if Indent > 0 then
         begin
            if ToString then
               Put(F, ' ')
            else
            begin
               WriteLn(F);
               for I := 1 to Indent * Depth do
                  Write(F, ' ');
            end;
         end;
      end;

   procedure WriteValue(var F: Text; V: PJsonValue; Indent, Depth: Integer);
      var
         Member: PJsonValue;
      begin
         case JsonKind(V) of
            jkNull: Put(F, 'null');
            jkBoolean:
               if V^.Bool then
                  Put(F, 'true')
               else
                  Put(F, 'false');
            jkNumber: Put(F, NumberText(V^.Number));
            jkString: Put(F, Quoted(V^.Str));
            jkArray, jkObject:
            begin
               if V^.Kind = jkArray then
                  Put(F, '[')
               else
                  Put(F, '{');
               Member := V^.First;
               while Member <> nil do
               begin
                  NewLine(F, Indent, Depth + 1);
                  if V^.Kind = jkObject then
                  begin
                     Put(F, Quoted(Member^.Key));
                     if Indent > 0 then
                        Put(F, ': ')
                     else
                        Put(F, ':');
                  end;
                  WriteValue(F, Member, Indent, Depth + 1);
                  Member := Member^.Next;
                  if Member <> nil then
                     Put(F, ',');
               end;
               if V^.First <> nil then
                  NewLine(F, Indent, Depth);
               if V^.Kind = jkArray then
                  Put(F, ']')
               else
                  Put(F, '}');
            end;
         end;
      end;

   procedure JsonWrite(var F: Text; V: PJsonValue; Indent: Integer);
      begin
         ToString := False;
         WriteValue(F, V, Indent, 0);
      end;

   procedure JsonSave(const FileName: string; V: PJsonValue; Indent: Integer);
      var
         F: Text;
      begin
         Assign(F, FileName);
         Rewrite(F);
         JsonWrite(F, V, Indent);
         WriteLn(F);
         Close(F);
      end;

   function JsonToString(V: PJsonValue): string;
      begin
         ToString := True;
         Output2 := '';
         WriteValue(Output, V, 0, 0);
         ToString := False;
         JsonToString := Output2;
      end;

end.
