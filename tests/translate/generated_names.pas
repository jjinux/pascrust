program GeneratedNames;
{ A program's own names that match names the generated Rust uses by
  itself: a type Vec (typed files encode into a Vec<u8>), a type Default
  (object hierarchies implement Default), and a method TypeName (TypeOf
  adds a type_name method); object methods Clone and Default (objects
  derive Clone and Default), a field Base in a descendant (which keeps
  its parent's part in base), and a type AnyShape (the enum of a
  hierarchy rooted at TShape). }
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
    base: Integer;
    function TypeName: string; virtual;
    procedure Clone;
    procedure Default;
  end;
  AnyShape = record
    kind: Integer;
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

procedure TSquare.Clone;
begin
  side := side * 2;
end;

procedure TSquare.Default;
begin
  base := 10;
end;

var
  f: file of Rec;
  r: Rec;
  d: Default;
  p: PShape;
  sq: TSquare;
  any: AnyShape;
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
  sq.Init(3);
  sq.Clone;
  sq.Default;
  any.kind := 2;
  WriteLn(sq.side, ' ', sq.base, ' ', any.kind, ' ', Pi:0:2);
end.
