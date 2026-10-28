# Plan: virtual methods

Status: not started. `virtual` is a parse error ("virtual methods are not
supported yet"); `constructor` and `destructor` parse as plain procedures.
None of the examples need it. This note works out what supporting it would
take, so we can decide whether it's worth it.

## What Turbo Pascal does

```pascal
type
  PShape = ^TShape;
  TShape = object
    x, y: Integer;
    constructor Init(ax, ay: Integer);
    procedure Draw; virtual;
    procedure Show;            { static: calls Draw }
    destructor Done; virtual;
  end;
  PCircle = ^TCircle;
  TCircle = object(TShape)
    radius: Integer;
    constructor Init(ax, ay, r: Integer);
    procedure Draw; virtual;   { overrides TShape.Draw }
  end;

procedure TShape.Show; begin Draw end;
```

Dynamic dispatch shows up in three places, from most to least common in
student code:

1. **Through `Self`, on a plain variable.** `var c: TCircle; c.Init(1, 2, 3);
   c.Show;` runs the inherited, static `TShape.Show`, whose `Draw` calls
   `TCircle.Draw`. No pointers are involved, and `c`'s type is known.
2. **Through a `var` parameter** of the base type: `procedure Paint(var s:
   TShape); begin s.Draw end;` called with a `TCircle`.
3. **Through pointers:** `p: PShape; p := New(PCircle, Init(1, 2, 3));
   p^.Draw; Dispose(p, Done);`. A `PCircle` may be assigned to a `PShape`.
   This is the classic "list of shapes" program.

TP also has `New(P, Init)` and `Dispose(P, Done)` (the extended syntax), and
`New` as a function, `p := New(PCircle, Init)`. A constructor sets up the
object's VMT pointer; calling a virtual method before it is a runtime error
under `{$R+}` and a crash otherwise.

## Where we are

`TCircle` is `struct TCircle { base: TShape, radius: i16 }` with
`impl TCircle { fn draw(&mut self) ... }`; inherited members are reached as
`self.base.x` / `self.base.show()`. `self.base.show()` runs with
`self: &mut TShape`, so inside `show` there is no way back to the circle:
that is exactly what case 1 needs.

## Design

One trait per hierarchy (per root object with a virtual method), holding
the virtual methods *and* every method that calls one, directly or through
other methods (`Show`). Each object in the hierarchy implements it:

```rust
/// The virtual methods of `TShape` and the objects descended from it.
trait Shape {
    /// The `TShape` part of the object, for its fields.
    fn shape(&mut self) -> &mut TShape;
    fn draw(&mut self);
    /// Static in Pascal, but it calls `draw`, so it must see the real object.
    fn show(&mut self) {
        self.draw();
    }
}

impl Shape for TShape {
    fn shape(&mut self) -> &mut TShape { self }
    fn draw(&mut self) { ... }
}

impl Shape for TCircle {
    fn shape(&mut self) -> &mut TShape { &mut self.base }
    fn draw(&mut self) { ... self.shape().x ... self.radius ... }
}
```

- **Case 1** falls out: `c.show()` is the trait's default method with
  `Self = TCircle`, so its `self.draw()` is `TCircle::draw`. Fields of the
  root inside default methods go through `self.shape().x`. Static dispatch,
  no `dyn`, no allocation. `inherited Draw` is `self.base.draw()` (or
  `TShape::draw(&mut self.base)`).
- **Case 2:** a `var s: TShape` parameter that the procedure uses for a
  virtual call becomes `s: &mut dyn Shape`. Passing a `TCircle` is
  `&mut circle`, and passing a `TShape` still works. Fields through it:
  `s.shape().x`.
- **Case 3:** pointers into the hierarchy become `Ptr<dyn Shape>`. That needs
  a runtime change: `Ptr<T>` is `Rc<RefCell<Option<T>>>`, and `Option<T>`
  needs a sized `T`. Making the slot `Rc<RefCell<Box<T>>>` with
  `T: ?Sized` (and `nil`/disposed tracked outside the cell) allows
  `Ptr<dyn Shape>`. `New(PCircle, Init)` allocates a `TCircle` and
  coerces it. Assigning a `PCircle` to a `PShape` is then free if *both*
  are `Ptr<dyn Shape>`, but then `pc^.radius` needs a downcast
  (`as_any().downcast_mut::<TCircle>()`), which is the ugly part. The
  alternative, keeping `PCircle = Ptr<TCircle>` and converting on
  assignment, loses Pascal's sharing (both pointers must reach one record).

## Cost

- Case 1 (and `constructor`/`destructor`/`inherited` on plain variables):
  sema computes, per hierarchy, which methods must move into the trait (a
  fixed point over the call graph). Codegen emits the trait, the impls and
  `self.shape()` paths in default methods. About a day's work, mostly in
  `codegen/mod.rs` (object emission) and `sema/check.rs` (method
  resolution), plus tests. Medium.
- Case 2: small once case 1 exists (a parameter type change and
  `&mut dyn`).
- Case 3: the runtime `Ptr` change (it touches every pointer program, so it
  needs care and benchmarks of the existing tests), the extended `New`/
  `Dispose` syntax, and the downcast for descendant fields. Several more
  days, and the generated code gets noticeably less pleasant.

## Recommendation

Do case 1 and case 2 together if a real program needs virtual methods;
leave case 3 until a program with a polymorphic list turns up, since that's
where most of the cost and the least readable Rust are.
