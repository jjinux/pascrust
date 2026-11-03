{ PRJson, PascRust's own unit for JSON: parsing, looking at values,
  building them, writing them compactly and indented, a round trip
  through a file, and errors. Output from fpc -Mtp (PRJson is Turbo
  Pascal). }
program PRJsonTest;
uses PRJson;
var
   Doc, People, P: PJsonValue;
   I: Integer;
   F: Text;

procedure Show(const S: string);
   var
      V: PJsonValue;
   begin
      V := JsonParse(S);
      if V = nil then
         WriteLn('error: ', JsonError)
      else
      begin
         WriteLn(JsonToString(V));
         JsonFree(V);
      end;
   end;

begin
   Doc := JsonParse('{"name": "Ann", "age": 37, "tags": ["a", "b\n"], "ok": true, "none": null, "pi": 3.25}');
   WriteLn(JsonGetString(Doc, 'name'), ' ', JsonGetInteger(Doc, 'age'), ' ', JsonCount(JsonGet(Doc, 'tags')),
      ' ', JsonGetBool(Doc, 'ok'), ' ', JsonKind(JsonGet(Doc, 'none')) = jkNull);
   WriteLn('missing: [', JsonGetString(Doc, 'nope'), '] ', JsonGetInteger(Doc, 'nope'), ' ', JsonGet(Doc, 'nope') = nil);
   WriteLn('second tag: ', JsonString(JsonItem(JsonGet(Doc, 'tags'), 1)) = 'b'#10);
   JsonWrite(Output, Doc, 0);
   WriteLn;
   JsonWrite(Output, Doc, 2);
   WriteLn;
   JsonFree(Doc);
   { Numbers, escapes, nesting, empty lists. }
   Show('[-0.25, 1e3, 12.5E-1, 0, -7]');
   Show('"tab\tquote\" back\\ slash\/ Aé"');
   Show('{"a": {"b": [[], {}]}, "c": []}');
   Show('  true  ');
   { \u escapes above 255 (a surrogate pair too) read as '?'. }
   Show('"' + '\' + 'u0041' + '\' + 'u20ac' + '\' + 'ud83d' + '\' + 'ude00"');
   { Errors. }
   Show('[1e400]');
   Show('[1, 2,');
   Show('{"a" 1}');
   Show('{a: 1}');
   Show('[1] 2');
   Show('tru');
   Show('"\q"');
   { Building: JsonSet replaces a member in place. }
   People := JsonNewArray;
   for I := 1 to 3 do
   begin
      P := JsonNewObject;
      JsonSet(P, 'id', JsonNewNumber(I));
      JsonSet(P, 'half', JsonNewNumber(I / 2));
      JsonAdd(People, P);
   end;
   JsonSet(JsonItem(People, 1), 'id', JsonNewString('two'));
   JsonAdd(People, JsonNewBool(False));
   JsonAdd(People, JsonNewNull);
   WriteLn(JsonToString(People));
   { A round trip through a file. }
   JsonSave('pr_json.json', People, 3);
   JsonFree(People);
   Doc := JsonLoad('pr_json.json');
   WriteLn(JsonCount(Doc), ' ', JsonGetString(JsonItem(Doc, 1), 'id'), ' ', JsonGetNumber(JsonItem(Doc, 2), 'half'):0:1);
   JsonWrite(Output, Doc, 3);
   WriteLn;
   JsonFree(Doc);
   Assign(F, 'pr_json.json');
   Erase(F);
   { A string whose escaped form is longer than 255 characters is written
     whole, and reads back. }
   Doc := JsonNewArray;
   P := JsonNewString('');
   for I := 1 to 150 do
      P^.Str := P^.Str + '"';
   JsonAdd(Doc, P);
   JsonSave('pr_json.json', Doc, 0);
   JsonFree(Doc);
   Doc := JsonLoad('pr_json.json');
   WriteLn('150 quotes: ', Length(JsonString(JsonItem(Doc, 0))));
   JsonFree(Doc);
   Erase(F);
   Doc := JsonLoad('no_such_file.json');
   WriteLn(Doc = nil, ' ', JsonError);
end.
