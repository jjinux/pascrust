program GeneratedNames;
{ A program's own names that match names the generated Rust uses by
  itself: a type Vec (typed files encode into a Vec<u8>), a type Default
  (object hierarchies implement Default), and a method TypeName (TypeOf
  adds a type_name method). }
type
  Vec = record
    x, y: Real;
  end;
  Rec = record
    a: Integer;
    v: Vec;
  end;
  Default = record
    n: Integer;
  end;
  PShape = ^TShape;
  TShape = object
    side: Integer;
    constructor Init(s: Integer);
    function TypeName: string; virtual;
  end;
  TSquare = object(TShape)
    function TypeName: string; virtual;
  end;

constructor TShape.Init(s: Integer);
begin
  side := s;
end;

function TShape.TypeName: string;
begin
  TypeName := 'shape';
end;

function TSquare.TypeName: string;
begin
  TypeName := 'square';
end;

var
  f: file of Rec;
  r: Rec;
  d: Default;
  p: PShape;
begin
  r.a := 7;
  r.v.x := 1.5;
  r.v.y := 2.5;
  Assign(f, 'gnames.dat');
  Rewrite(f);
  Write(f, r);
  Close(f);
  r.a := 0;
  Reset(f);
  Read(f, r);
  Close(f);
  Erase(f);
  WriteLn(r.a, ' ', r.v.x:3:1, ' ', r.v.y:3:1);
  d.n := 3;
  p := New(PShape, Init(4));
  WriteLn(d.n, ' ', p^.TypeName, ' ', TypeOf(p^) = TypeOf(TShape), ' ', p^.side);
  Dispose(p);
end.
