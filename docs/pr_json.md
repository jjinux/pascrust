# PRJson: reading and writing JSON

`PRJson` is one of PascRust's own units (their names start with `PR`).
It reads and writes JSON from Turbo Pascal. Like `PRTurtle`, it's
written in Turbo Pascal (`crates/pascrust/units/PRJson.pas`), comes with
PascRust (`uses PRJson` finds it unless there's a file of that name next
to the program), and also compiles in Turbo Pascal 7 and Free Pascal.

```pascal
program People;
uses PRJson;
var
   Doc, Person: PJsonValue;
   I: Integer;
begin
   Doc := JsonLoad('people.json');
   if Doc = nil then
   begin
      WriteLn('people.json: ', JsonError);
      Halt(1);
   end;
   for I := 0 to JsonCount(Doc) - 1 do
   begin
      Person := JsonItem(Doc, I);
      WriteLn(JsonGetString(Person, 'name'), ' is ', JsonGetInteger(Person, 'age'));
   end;
   JsonFree(Doc);
end.
```

## Values

A JSON value is a record on the heap, reached through a `PJsonValue`
(`^TJsonValue`, in Turbo Pascal's naming). `TJsonValue.Kind` says what
it is: `jkNull`, `jkBoolean`, `jkNumber`, `jkString`, `jkArray` or
`jkObject`. An array's or object's members are a linked list (`First`,
`Next`), so a program can walk them itself as well as use the functions
below. Free a value with `JsonFree` when you're done; it frees
everything in it.

| Routine | What it does |
|---|---|
| `JsonParse(S)`, `JsonLoad(FileName)` | Read JSON from a string or a file: the value, or `nil` with the reason in `JsonError` (like `'expected '':'' at character 6'`). |
| `JsonKind(V)` | What `V` is (`jkNull` for `nil` too). |
| `JsonCount(V)` | How many members an array or object has. |
| `JsonItem(V, I)` | An array's member `I`, counting from 0 (as `TCollection.At` does), or `nil`. |
| `JsonGet(V, Key)` | An object's member `Key`, or `nil`. |
| `JsonString(V)`, `JsonNumber(V)`, `JsonInteger(V)`, `JsonBool(V)` | A value as a Pascal one; `''`, `0` or `False` if it's something else. |
| `JsonGetString(V, Key)` and so on | The same for an object's member: `JsonString(JsonGet(V, Key))`. |
| `JsonNewNull`, `JsonNewBool(B)`, `JsonNewNumber(X)`, `JsonNewString(S)`, `JsonNewArray`, `JsonNewObject` | New values. |
| `JsonAdd(Arr, V)` | Put `V` at the end of an array. |
| `JsonSet(Obj, Key, V)` | Set an object's member, replacing (and freeing) one it had. |
| `JsonWrite(F, V, Indent)` | Write `V` to a text file (`Output` too): on one line if `Indent` is 0, else one member a line, indented `Indent` spaces a level. |
| `JsonSave(FileName, V, Indent)` | `JsonWrite` to a new file. |
| `JsonToString(V)` | `V` as compact JSON in a string (cut at 255 characters; `JsonWrite` and `JsonSave` write any length). |
| `JsonFree(V)` | Free `V` and everything in it. |

## Limits

Turbo Pascal's strings hold 255 characters, so a longer JSON string or
key is cut to 255 (a whole file can be any size). Characters above 127
pass through as bytes, so UTF-8 text comes back out as it went in; a
`\u` escape up to `\u00ff` reads as that character, and one above as
`?`. Numbers are `Real`s, so about 11 significant digits survive
(`3.14159265358979` comes back as `3.141592654`), and one too big for a
`Real` (beyond about 1.7e38, as in TP: `1e400`) is an error. A whole number is written without a point
(`37`), others with up to nine decimals and no trailing zeros (`0.5`; so
`1e-10` is written as `0`). `JsonAdd` to something that isn't an array,
or `JsonSet` on something that isn't an object, frees the value.
