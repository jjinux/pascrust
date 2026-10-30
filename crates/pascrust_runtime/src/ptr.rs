//! `Ptr<T>`: Pascal pointers, made safe.
//!
//! A Pascal pointer is an address: `New(p)` allocates a record on the heap,
//! `p^` reaches it, several pointers may hold the same address, and
//! `Dispose(p)` frees it, after which any other pointer to it dangles. That's
//! exactly what Rust forbids, so instead of an address, a `Ptr<T>` holds a
//! *handle* to a shared heap cell:
//!
//! - `Rc` (reference counting) lets several `Ptr`s share one cell; the cell
//!   is freed when the last handle goes away.
//! - `RefCell` lets any of them change the record. Rust normally checks at
//!   compile time that nobody reads a value while it's being changed;
//!   `RefCell` does that check at run time instead, because with pointers the
//!   compiler can't know who points where.
//! - The `Option` inside is the record itself, or `None` once disposed, so
//!   a dangling pointer is a clean runtime error rather than a crash.
//!
//! This is the "virtual heap" idea from the project's planning notes: the
//! Pascal heap lives inside Rust, and misuse raises an error to the host
//! instead of corrupting memory.

use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::rc::Rc;

pub struct Ptr<T>(Option<Rc<RefCell<Option<T>>>>);

impl<T> Ptr<T> {
    /// Pascal `nil`.
    pub const fn nil() -> Ptr<T> {
        Ptr(None)
    }

    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// A new heap record initialized to `value`.
    pub fn new(value: T) -> Ptr<T> {
        Ptr(Some(Rc::new(RefCell::new(Some(value)))))
    }

    /// Pascal `Dispose(p)`: free the record and set `p` to `nil`. Other
    /// pointers to the same record become dangling: using them is a runtime
    /// error.
    pub fn dispose(&mut self) {
        match self.0.take() {
            Some(cell) => match cell.try_borrow_mut() {
                Ok(mut slot) => {
                    if slot.take().is_none() {
                        crate::rte(204, "Invalid pointer operation: record disposed twice");
                    }
                }
                Err(_) => crate::rte(204, "Invalid pointer operation: disposing a record that's in use"),
            },
            None => crate::rte(204, "Invalid pointer operation: Dispose(nil)"),
        }
    }

    fn cell(&self) -> &RefCell<Option<T>> {
        match &self.0 {
            Some(cell) => cell,
            None => crate::rte(204, "Invalid pointer operation: dereferencing nil"),
        }
    }

    /// Pascal `p^` for reading. The returned guard keeps the record borrowed
    /// until it's dropped, at the end of the statement.
    pub fn get(&self) -> Ref<'_, T> {
        let guard = match self.cell().try_borrow() {
            Ok(guard) => guard,
            Err(_) => crate::rte(204, ALIASED),
        };
        // `Ref::map` narrows a borrow of the `Option<T>` to a borrow of the
        // `T` inside it.
        Ref::map(guard, |slot| match slot {
            Some(value) => value,
            None => crate::rte(204, "Invalid pointer operation: using a disposed record"),
        })
    }

    /// Pascal `p^` for writing (`p^.x := 1`).
    pub fn get_mut(&self) -> RefMut<'_, T> {
        let guard = match self.cell().try_borrow_mut() {
            Ok(guard) => guard,
            Err(_) => crate::rte(204, ALIASED),
        };
        RefMut::map(guard, |slot| match slot {
            Some(value) => value,
            None => crate::rte(204, "Invalid pointer operation: using a disposed record"),
        })
    }

    /// Run `f` with the record borrowed mutably once, for a call that
    /// passes two of its fields by `var`: `SetTwo(q^.x, q^.y)` is
    /// `q.with_mut(|point| set_two(&mut point.x, &mut point.y))`. Calling
    /// `get_mut()` twice would borrow the `RefCell` twice; one `&mut T` can
    /// be split into its fields, because Rust sees they are disjoint.
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        // `&mut *guard` turns the `RefMut` guard into a plain `&mut T`.
        f(&mut *self.get_mut())
    }
}

/// The message for a `RefCell` conflict. In Pascal, two pointers to the same
/// record (or a `var` parameter that is a field of the record) may be used at
/// once; in Rust's model, one of them is still "in use" when the other
/// writes.
const ALIASED: &str = "Invalid pointer operation: a heap record is used through two references at once \
    (e.g. passed as a `var` argument while another pointer changes it)";

impl<T: Default> Ptr<T> {
    /// Pascal `New(p)`: a new heap record. Turbo Pascal leaves its contents
    /// undefined; ours starts out zeroed (`Default`).
    pub fn alloc() -> Ptr<T> {
        Ptr::new(T::default())
    }
}

/// Copying a pointer copies the handle, not the record: both point at the
/// same cell, as in Pascal.
impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Ptr<T> {
        Ptr(self.0.clone())
    }
}

impl<T> Default for Ptr<T> {
    fn default() -> Ptr<T> {
        Ptr::nil()
    }
}

/// Pointer equality is identity: do they point at the same record?
impl<T> PartialEq for Ptr<T> {
    fn eq(&self, other: &Ptr<T>) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T> fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(cell) => write!(f, "Ptr({:p})", Rc::as_ptr(cell)),
            None => write!(f, "nil"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Clone)]
    struct Node {
        value: i16,
        next: Ptr<Node>,
    }

    #[test]
    fn linked_list() {
        let mut head: Ptr<Node> = Ptr::nil();
        for v in 1..=3 {
            let p: Ptr<Node> = Ptr::alloc();
            p.get_mut().value = v;
            p.get_mut().next = head.clone();
            head = p;
        }
        let mut sum = 0;
        let mut p = head.clone();
        while !p.is_nil() {
            sum += p.get().value;
            let next = p.get().next.clone();
            p = next;
        }
        assert_eq!(sum, 6);
    }

    #[test]
    fn sharing_and_identity() {
        let a: Ptr<Node> = Ptr::alloc();
        let b = a.clone();
        b.get_mut().value = 7;
        assert_eq!(a.get().value, 7);
        assert!(a == b);
        assert!(a != Ptr::alloc());
    }

    #[test]
    #[should_panic]
    fn dangling_pointer_is_an_error() {
        let mut a: Ptr<Node> = Ptr::alloc();
        let b = a.clone();
        a.dispose();
        let _ = b.get().value;
    }

    #[test]
    #[should_panic]
    fn nil_dereference_is_an_error() {
        let p: Ptr<Node> = Ptr::nil();
        let _ = p.get().value;
    }
}

/// Pascal's untyped `Pointer`: `nil`, a block of bytes from `GetMem`, or any
/// typed pointer with its type forgotten (`p := Node1`, then `PNode(p)^`).
/// Turbo Pascal's `Pointer` was just an address; here the type a pointer
/// had is kept out of sight in an `Rc<dyn Any>`, and `PNode(p)` asks for it
/// back (`p.cast::<Node>()`), which is a runtime error if it was something
/// else: TP would have read the bytes as the wrong type.
#[derive(Clone, Default)]
pub enum AnyPtr {
    #[default]
    Nil,
    /// `GetMem(p, Size)`: `Size` bytes, which `p^` sees as untyped memory.
    Bytes(Rc<RefCell<Vec<u8>>>),
    /// A typed pointer's record cell, its type erased. (`dyn Any` is Rust's
    /// way of holding "a value of some type", which `downcast` recovers.)
    Typed(Rc<dyn std::any::Any>),
    /// The address of code: `@Handler` of a procedure, or an interrupt
    /// vector's original handler (see `GetIntVec`). Only kept and compared:
    /// PascRust never calls through it.
    Code(Rc<str>),
}

impl AnyPtr {
    pub const fn nil() -> AnyPtr {
        AnyPtr::Nil
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, AnyPtr::Nil)
    }

    /// `GetMem(p, Size)` for a `Pointer`: `Size` zero bytes.
    pub fn alloc(size: impl Into<i32>) -> AnyPtr {
        let size = usize::try_from(size.into()).unwrap_or(0);
        AnyPtr::Bytes(Rc::new(RefCell::new(vec![0; size])))
    }

    /// `FreeMem(p, Size)`: the bytes are gone; using them again is runtime
    /// error 201. (As in TP, `p` itself is unchanged.)
    pub fn free(&mut self) {
        match self {
            AnyPtr::Bytes(bytes) => bytes.borrow_mut().clear(),
            AnyPtr::Nil => crate::rte(204, "Invalid pointer operation: FreeMem(nil)"),
            AnyPtr::Typed(_) | AnyPtr::Code(_) => {}
        }
    }

    /// `p := Node1`: any typed pointer, its type forgotten.
    pub fn from_ptr<T: 'static>(pointer: &Ptr<T>) -> AnyPtr {
        match &pointer.0 {
            Some(cell) => AnyPtr::Typed(cell.clone()),
            None => AnyPtr::Nil,
        }
    }

    /// `PNode(p)`, or `Node1 := p`: the typed pointer back.
    pub fn cast<T: 'static>(&self) -> Ptr<T> {
        match self {
            AnyPtr::Nil => Ptr::nil(),
            AnyPtr::Typed(cell) => match cell.clone().downcast::<RefCell<Option<T>>>() {
                Ok(cell) => Ptr(Some(cell)),
                Err(_) => crate::rte(204, "Invalid pointer operation: a Pointer used as a pointer to another type"),
            },
            AnyPtr::Bytes(_) => crate::rte(204, "Invalid pointer operation: memory from GetMem used as a typed record"),
            AnyPtr::Code(_) => crate::rte(204, "Invalid pointer operation: a procedure's address used as a record"),
        }
    }

    /// `@Handler`: a procedure's address, by its name.
    pub fn code(name: &str) -> AnyPtr {
        AnyPtr::Code(Rc::from(name))
    }

    /// `p^` as untyped memory (`BlockRead(f, p^, n)`): its bytes.
    pub fn bytes(&self) -> BufferView {
        match self {
            AnyPtr::Bytes(bytes) => BufferView(bytes.clone()),
            AnyPtr::Nil => crate::rte(204, "Invalid pointer operation: dereferencing nil"),
            AnyPtr::Typed(_) => crate::rte(204, "Invalid pointer operation: a typed record used as untyped memory"),
            AnyPtr::Code(_) => crate::rte(204, "Invalid pointer operation: a procedure's address used as memory"),
        }
    }
}

/// Two `Pointer`s are equal when they point to the same thing.
impl PartialEq for AnyPtr {
    fn eq(&self, other: &AnyPtr) -> bool {
        match (self, other) {
            (AnyPtr::Nil, AnyPtr::Nil) => true,
            (AnyPtr::Bytes(a), AnyPtr::Bytes(b)) => Rc::ptr_eq(a, b),
            (AnyPtr::Typed(a), AnyPtr::Typed(b)) => Rc::ptr_eq(a, b),
            (AnyPtr::Code(a), AnyPtr::Code(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for AnyPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyPtr::Nil => write!(f, "nil"),
            AnyPtr::Bytes(bytes) => write!(f, "Pointer({} bytes)", bytes.borrow().len()),
            AnyPtr::Typed(_) => write!(f, "Pointer(..)"),
            AnyPtr::Code(name) => write!(f, "@{name}"),
        }
    }
}

/// As for `Ptr`, a stored pointer is 4 zero bytes and reads back as nil.
impl crate::io::FileRecord for AnyPtr {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[0; 4]);
    }
    fn decode(input: &mut &[u8]) -> Self {
        *input = &input[4.min(input.len())..];
        AnyPtr::Nil
    }
}

/// The bytes a `Pointer` from `GetMem` points to, as untyped memory.
pub struct BufferView(Rc<RefCell<Vec<u8>>>);

impl crate::bytes::ByteView for BufferView {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.borrow().clone()
    }

    fn set_bytes(&mut self, bytes: &[u8]) {
        let mut buffer = self.0.borrow_mut();
        let n = bytes.len().min(buffer.len());
        buffer[..n].copy_from_slice(&bytes[..n]);
    }
}

#[cfg(test)]
mod any_ptr_tests {
    use super::*;
    use crate::bytes::ByteView;

    #[test]
    fn a_pointer_keeps_its_type_out_of_sight() {
        let node: Ptr<i16> = Ptr::new(5);
        let any = AnyPtr::from_ptr(&node);
        assert_eq!(*any.cast::<i16>().get(), 5);
        assert!(any == AnyPtr::from_ptr(&node));
        assert!(AnyPtr::nil().cast::<i16>().is_nil());
    }

    #[test]
    fn getmem_gives_bytes() {
        let p = AnyPtr::alloc(4);
        p.bytes().set_bytes(&[1, 2]);
        assert_eq!(p.bytes().to_bytes(), [1, 2, 0, 0]);
    }
}

/// A Turbo Pascal procedural variable: a routine, or nil. `F` is the
/// function-pointer type (`fn(f64) -> f64`), so a `ProcVar` is `Copy` and
/// calling one is an ordinary call: `f.get()(x)`. It keeps the routine's
/// name too, which is what `@F = @Square` compares: Rust warns that two
/// function pointers to the same code may not compare equal.
#[derive(Clone, Copy)]
pub struct ProcVar<F: Copy> {
    routine: Option<F>,
    name: &'static str,
}

impl<F: Copy> ProcVar<F> {
    pub const fn nil() -> Self {
        ProcVar { routine: None, name: "" }
    }

    /// `F := Square`.
    pub const fn of(routine: F, name: &'static str) -> Self {
        ProcVar { routine: Some(routine), name }
    }

    /// `F = nil`, `not Assigned(F)`.
    pub fn is_nil(&self) -> bool {
        self.routine.is_none()
    }

    /// The routine, to call. Calling a nil procedural variable is runtime
    /// error 210, where TP jumped to address 0.
    pub fn get(&self) -> F {
        match self.routine {
            Some(routine) => routine,
            None => crate::rte(210, "Call of a nil procedural variable"),
        }
    }

    /// `@F` as a `Pointer`: the routine's address, compared by name.
    pub fn address(&self) -> AnyPtr {
        if self.is_nil() { AnyPtr::Nil } else { AnyPtr::code(self.name) }
    }
}

impl<F: Copy> Default for ProcVar<F> {
    fn default() -> Self {
        ProcVar::nil()
    }
}

/// Two procedural values are equal when they're the same routine (or both
/// nil).
impl<F: Copy> PartialEq for ProcVar<F> {
    fn eq(&self, other: &Self) -> bool {
        self.routine.is_some() == other.routine.is_some() && self.name == other.name
    }
}

impl<F: Copy> fmt::Debug for ProcVar<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() { write!(f, "nil") } else { write!(f, "@{}", self.name) }
    }
}

/// As for other pointers, a stored procedural value is 4 zero bytes and
/// reads back as nil.
impl<F: Copy> crate::io::FileRecord for ProcVar<F> {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[0; 4]);
    }
    fn decode(input: &mut &[u8]) -> Self {
        *input = &input[4.min(input.len())..];
        ProcVar::nil()
    }
}
