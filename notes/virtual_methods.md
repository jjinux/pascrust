# Plan: virtual methods

Status: implemented (2026-10-28), as designed below, in well under the
2-hour time box. Not done: `TypeOf`, a descendant hiding a static method
that calls virtual ones (an error), a procedure nested in a method calling
a virtual method of `Self` (an error). Tests: `tests/run/virtual_*.pas`.
The design was worked out by Claude Opus and reviewed by Claude Fable,
whose findings are folded in below.

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
   `TCircle.Draw`.
2. **Through a `var` parameter** of the base type: `procedure Paint(var s:
   TShape); begin s.Draw end;` called with a `TCircle`.
3. **Through pointers:** `p: PShape; p := New(PCircle, Init(1, 2, 3));
   p^.Draw; Dispose(p, Done);`. A `PCircle` may be assigned to a `PShape`,
   both may point at one record, and `pc^.radius` reaches the descendant's
   fields. This is the classic "list of shapes" program.

TP also has the extended `New(P, Init)` / `Dispose(P, Done)` syntax, `New`
as a function (`p := New(PCircle, Init)`), `inherited Draw`, and `TypeOf`.
A constructor sets the object's VMT pointer; a virtual call before it is
runtime error 210 under `{$R+}` and a crash otherwise.

## Where we are

`TCircle` is `struct TCircle { base: TShape, radius: i16 }` with
`impl TCircle { fn draw(&mut self) ... }`; inherited members are reached as
`self.base.x` / `self.base.show()` (architecture.md §16). Inside
`self.base.show()`, `self` is a `&mut TShape`, so there's no way back to the
circle, which is exactly what case 1 needs. Pointers are `Ptr<T>`, a handle
over `Rc<RefCell<Option<T>>>` (§12), which holds exactly one `T`: a `TCircle`
doesn't fit in a `Ptr<TShape>`.

## Design: a trait for dispatch, an enum for storage

One **trait** per hierarchy (per root object that has a virtual method)
handles dispatch, cases 1 and 2. One **enum** per hierarchy is what every
pointer into it points to, case 3. The two meet in `impl Shape for
AnyShape`, which makes them one design rather than two.

```rust
/// The virtual methods of `TShape` and the objects descended from it, plus
/// the methods that call them (which must see the real object).
trait Shape {
    /// The `TShape` part of the object, for its fields.
    fn shape(&self) -> &TShape;
    fn shape_mut(&mut self) -> &mut TShape;
    fn draw(&mut self); // virtual
    /// Static in Pascal, but it calls `draw`, so it's a default method:
    /// `self` is the real object, and `self.draw()` runs its version.
    fn show(&mut self) {
        self.draw();
    }
}

impl Shape for TShape {
    fn shape(&self) -> &TShape { self }
    fn shape_mut(&mut self) -> &mut TShape { self }
    fn draw(&mut self) { /* TShape.Draw */ }
}

impl Shape for TCircle {
    fn shape(&self) -> &TShape { &self.base }
    fn shape_mut(&mut self) -> &mut TShape { &mut self.base }
    fn draw(&mut self) { /* TCircle.Draw: self.base.x, self.radius */ }
}

/// Every object of the `TShape` hierarchy a pointer can reach. The variant
/// plays the role of Turbo Pascal's VMT pointer: it says which `draw` runs.
#[derive(Clone, Debug, Default)]
enum AnyShape {
    #[default]
    Shape(TShape),
    Circle(TCircle),
}

impl Shape for AnyShape {
    fn draw(&mut self) {
        match self {
            AnyShape::Shape(s) => s.draw(),
            AnyShape::Circle(c) => c.draw(),
        }
    }
    fn shape_mut(&mut self) -> &mut TShape {
        match self {
            AnyShape::Shape(s) => s,
            AnyShape::Circle(c) => &mut c.base,
        }
    }
    // `shape()` likewise; `show()` stays the default.
}

impl AnyShape {
    /// `pc^.radius`: this pointer is known to point to a circle.
    fn circle_mut(&mut self) -> &mut TCircle {
        match self {
            AnyShape::Circle(c) => c,
            _ => rte(204, "Invalid pointer operation: not a TCircle"),
        }
    }
}

type PShape = Ptr<AnyShape>;
type PCircle = Ptr<AnyShape>; // the same type; sema keeps Pascal's distinction
```

How each case comes out:

- **Case 1:** `c.show()` is the trait's default method with `Self =
  TCircle`, so its `self.draw()` is `TCircle::draw`. Static dispatch, no
  `dyn`. Root fields inside default methods are `self.shape_mut().x`.
- **Case 2:** a `var s: TShape` parameter that the procedure makes a virtual
  call on becomes `s: &mut dyn Shape`; passing a `TCircle` is `&mut circle`.
  (`&mut impl Shape` would read as well, but `dyn` keeps the procedure
  non-generic, which matters for procedural types and nested-procedure
  lifting.)
- **Case 3:** `p^.Draw` is `p.get_mut().draw()`, the enum's `match`;
  `p^.Show` is `p.get_mut().show()`, the trait default with `Self =
  AnyShape`, which calls the enum's `draw`. No downcasts, and `Ptr` doesn't
  change (an enum has a fixed size).
  - `p := pc` copies the handle; `PCircle(p)` is a no-op.
  - `pc^.radius` → `pc.get_mut().circle_mut().radius`. If the record isn't a
    circle (only possible after a bad typecast), it's a runtime error where
    TP would silently corrupt memory. Sema already enforces Pascal's pointer
    compatibility, so a correct program loses nothing from `PShape` and
    `PCircle` sharing a Rust type.
  - `New(PCircle, Init(1, 2, 3))` → `let p = Ptr::new(AnyShape::Circle(
    TCircle::default())); p.get_mut().circle_mut().init(1, 2, 3);`. A plain
    `New(p)` still works through `Default` (`#[default]` on the root).
  - `Dispose(p, Done)` → `p.get_mut().done(); p.dispose();`: two statements,
    so the borrow ends before `dispose` needs the record.
  - `TypeOf(p^) = TypeOf(TCircle)` → `matches!(*p.get(), AnyShape::Circle(_))`.
  - Since the variant is set when the record is allocated, "virtual call
    before `Init`" can't happen. That's a benign difference from TP.

Only hierarchies that have a virtual method and more than one member get a
trait and an enum; everything else stays as it is today.

Neither mechanism can do the whole job. The enum can't do case 2: a `TCircle`
variable has no `AnyShape` around it to pass without moving it into one. The
trait can't do case 3 without a `?Sized` `Ptr` and downcasts.

## Problems to handle

1. **Lifted globals and fixed signatures (the biggest hidden cost).** A
   method receives the globals it uses as extra parameters (`fn add(&mut
   self, log: &mut i16, item: i16)` in the `objects.pas` translate
   snapshot; the fixed point is in `crates/pascrust/src/sema/lift.rs`). A
   trait method has one signature, so a virtual method must take the
   *union* of what all its overrides need, and a virtual call must pass that
   union on to its caller's needs. The closed world makes this computable.
   An override that doesn't use one of them names it `_log` to stay
   warning-free.
2. **Units.** Codegen emits a type only in the file that declares it
   (`type_defs` in `crates/pascrust/src/codegen/mod.rs`), and units are
   `pub mod` + `use crate::x::*`. A descendant can be declared in a later
   unit than its root, so the enum (and `impl Shape for AnyShape`, and the
   pointer aliases) go in the root's module, next to the trait, with `use
   crate::circles::TCircle;`. A unit naming a unit that uses it is legal in
   Rust but needs a teaching comment. The root unit's Rust then depends on
   the program it's part of, which is fine with one Cargo project per
   program.
3. **`inherited` on a static method that calls a virtual one.** If
   `TCircle.Show` hides `Show` and calls `inherited Show`,
   `self.base.show()` would run the default with `Self = TShape`, so its
   `Draw` would be `TShape.Draw`, not the circle's. Fix: emit the parent's
   body under a name nothing overrides (a `t_shape_show` default method)
   and have `show` delegate to it. `inherited Draw` → `self.base.draw()` is
   already right.
4. **Assignment.** By value, `s := c` stays `s = c.base.clone()`
   (truncation; TP also copies the VMT word, a known footgun; document the
   difference). Through pointers, `p^ := q^` must copy only the base part:
   `*p.get_mut().shape_mut() = q.get().shape().clone()`, not the whole enum,
   which would turn a plain shape record into a circle.
5. **Borrowing.** `p.get_mut().draw()` holds the record's `RefMut` for the
   whole method, so a `Draw` that reaches its own record through another
   pointer (a global list, say) stops with runtime error 204. That's the
   hazard architecture.md §12 already accepts for `var` heap fields, but
   it's likelier with objects. The existing hoisting rule (copy the pointer,
   then call) covers the usual loop, `p^.Draw; p := p^.next`.

## Rejected alternatives

- **`Rc<RefCell<dyn Shape>>`**: `Option<dyn Shape>` isn't allowed, so
  disposal moves to a separate flag; making the wrapper convert from
  `Ptr<TCircle>` needs nightly Rust's `CoerceUnsized`, so upcasts become
  explicit calls; downcasts need `Any`; `p^ := q^` needs a dyn-clone; no
  `TypeOf`.
- **A hand-rolled table of `fn(&mut TShape)` pointers**: the functions can't
  reach the descendant's fields without `unsafe`, or the enum anyway.
- **An arena of object handles**: the enum with a worse pointer.
- **Per-class "as if it were the parent" methods**: they address dispatch,
  which the trait already handles (and which is how `inherited` comes out),
  not storage, which is the hard part of case 3.

## Cost and order

About 4 days for a person working by hand. For Claude Code it's more like
an afternoon: a few hours of session time, judging by how long `with`,
`{$I-}` and two-fields-as-`var`-arguments took (well under an hour each),
since this is several features of that size. The split, in the person-days
the review used:

- Sema, 1.5 days: hierarchy roots, the "calls a virtual" fixed point, the
  lifted-parameter union, pointer compatibility, extended `New`/`Dispose`,
  `TypeOf`.
- Trait codegen, 1 day: the trait and impls, `self.shape_mut().x` paths in
  default methods, the `inherited` fix.
- Enum codegen, 1 day: the enum, its accessors and pointer aliases,
  placement across units, the base-part copy.
- Tests and docs, half a day: architecture.md §12 and §16, this note,
  language_spec.md.

Build case 1 first, including the lifted-parameter union (the risky part),
then case 2 (small), then the enum for case 3.
