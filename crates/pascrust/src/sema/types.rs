//! Pascal types as sema understands them, and how each maps to Rust.

use std::fmt;
use std::rc::Rc;

/// The integer types of Turbo Pascal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntKind {
    ShortInt,
    Byte,
    Integer,
    Word,
    LongInt,
}

impl IntKind {
    pub fn rust(self) -> &'static str {
        match self {
            IntKind::ShortInt => "i8",
            IntKind::Byte => "u8",
            IntKind::Integer => "i16",
            IntKind::Word => "u16",
            IntKind::LongInt => "i32",
        }
    }

    pub fn range(self) -> (i64, i64) {
        match self {
            IntKind::ShortInt => (-128, 127),
            IntKind::Byte => (0, 255),
            IntKind::Integer => (-32768, 32767),
            IntKind::Word => (0, 65535),
            IntKind::LongInt => (i32::MIN as i64, i32::MAX as i64),
        }
    }

    /// The type `Swap` works in for an argument of this type, as Free
    /// Pascal does: a Byte is swapped as a Word (`Swap(b)` with `b = 1` is
    /// 256), a ShortInt as an Integer, and a LongInt's two halves swap.
    pub fn for_swap(self) -> IntKind {
        match self {
            IntKind::Byte | IntKind::Word => IntKind::Word,
            IntKind::ShortInt | IntKind::Integer => IntKind::Integer,
            IntKind::LongInt => IntKind::LongInt,
        }
    }

    /// The same for a constant. TP's compiler folds `Swap` of a constant as
    /// its low 16 bits, unsigned (`Swap(Value.Word)`, as reconstructed at
    /// turbopascal.org), so the result is a Word: `Swap(255)` is 65280 and
    /// `Swap(-1)` 65535. Beyond 16 bits, a LongInt's halves swap, as Free
    /// Pascal does (TP took only Integer and Word).
    pub fn for_swap_value(n: i64) -> IntKind {
        match n {
            -32768..=65535 => IntKind::Word,
            _ => IntKind::LongInt,
        }
    }

    /// `Swap(n)` worked out for a constant.
    pub fn swap_value(n: i64) -> i64 {
        match IntKind::for_swap_value(n) {
            IntKind::Word => i64::from((n as u16).swap_bytes()),
            _ => i64::from((n as u32).rotate_left(16) as i32),
        }
    }

    pub fn contains(self, value: i64) -> bool {
        let (lo, hi) = self.range();
        lo <= value && value <= hi
    }

    /// Turbo Pascal's rule for the type of `a + b` when both are integers:
    /// arithmetic happens in the smallest type that can hold both operand
    /// types, and never in less than 16 bits (`Byte + Byte` is an `Integer`).
    pub fn common(a: IntKind, b: IntKind) -> IntKind {
        use IntKind::*;
        match (a, b) {
            (LongInt, _) | (_, LongInt) => LongInt,
            (Word, Word | Byte) | (Byte, Word) => Word,
            // A signed type mixed with `Word` needs 32 bits to hold both.
            (Word, _) | (_, Word) => LongInt,
            _ => Integer,
        }
    }

    /// The smallest type for an integer literal, as Turbo Pascal picks it.
    pub fn for_literal(value: i64) -> IntKind {
        if IntKind::Integer.contains(value) { IntKind::Integer } else { IntKind::LongInt }
    }
}

/// A Pascal type.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int(IntKind),
    /// An integer literal. It has no fixed type yet; it takes the type of the
    /// other operand, or of the variable it's assigned to.
    IntLit(i64),
    /// `Real` (and `Double`, `Extended`), all mapped to Rust's `f64`.
    Real,
    Bool,
    Char,
    /// `string[cap]`; plain `string` has `cap` 255.
    Str {
        cap: u8,
    },
    /// A string literal. Like [`Type::IntLit`], it adapts to its context: a
    /// one-character literal can be a `Char`.
    StrLit(usize),
    /// An enumerated type, `(Red, Green, Blue)`.
    Enum(Rc<EnumType>),
    /// `array[lo..hi] of T`. A multi-dimensional array is an array of arrays.
    Array(Rc<ArrayType>),
    Record(Rc<RecordType>),
    /// A Turbo Pascal 5.5 `object` type.
    Object(Rc<ObjectType>),
    /// `^T`, a pointer.
    Pointer(Rc<PointerType>),
    /// `Text`, a file of lines.
    Text,
    /// `file of T`.
    File(Rc<Type>),
    /// `set of T`. The empty set literal `[]` is a `Set` of `Void`, which
    /// fits any set type.
    Set(Rc<Type>),
    /// The type of `nil`.
    Nil,
    /// What `TypeOf` returns: which object type something really is. (In
    /// TP, a pointer to the type's table of virtual methods.) It can only be
    /// compared.
    TypeInfo,
    /// An untyped `var` parameter (`var Data`): some variable, seen as its
    /// bytes. It can only be passed on or typecast (`TBytes(Data)`).
    Untyped,
    /// The "type" of a procedure call, which has no value.
    Void,
    /// Produced after a type error so that one mistake doesn't cascade into
    /// many error messages.
    Error,
}

// The structured types live behind `Rc` (a reference-counted pointer), so
// cloning a `Type` is cheap even for a big record: it just bumps a count.

#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    /// The Rust name of the generated `enum`.
    pub rust_name: String,
    /// The Rust names of the variants, in order.
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType {
    /// The index type (an integer type, `Char`, `Boolean` or an enum).
    pub index: Type,
    pub lo: i64,
    pub hi: i64,
    pub elem: Type,
    /// The Rust type alias generated for a named array type (`GridT`).
    pub alias: Option<String>,
}

impl ArrayType {
    pub fn count(&self) -> i64 {
        self.hi - self.lo + 1
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordType {
    pub rust_name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The name as declared, for case-insensitive lookup and messages.
    pub name: String,
    pub rust_name: String,
    pub ty: Type,
}

/// `^T`. Pascal lets a pointer type name a type declared *later* in the
/// same `type` section (that's how linked lists are declared:
/// `NodePtr = ^Node; Node = record next: NodePtr; ... end`), so the target
/// starts out unresolved and is filled in once the section is done.
///
/// `RefCell` allows that late filling-in through a shared `Rc`. The type is
/// recursive (a node contains a pointer to a node), so `Debug` and
/// `PartialEq` look only at the target's name, not its contents, which would
/// recurse forever.
pub struct PointerType {
    pub target_name: String,
    pub target: std::cell::RefCell<Option<Type>>,
    /// The Rust type alias generated for a named pointer type.
    pub alias: Option<String>,
}

impl PointerType {
    pub fn target(&self) -> Type {
        self.target.borrow().clone().unwrap_or(Type::Error)
    }
}

impl PartialEq for PointerType {
    fn eq(&self, other: &PointerType) -> bool {
        self.target_name.eq_ignore_ascii_case(&other.target_name)
    }
}

impl fmt::Debug for PointerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "^{}", self.target_name)
    }
}

/// An object type. Its methods are recorded in the [`super::Analysis`]
/// (`methods`), because they're procedures with bodies of their own.
#[derive(Debug, Clone)]
pub struct ObjectType {
    pub rust_name: String,
    /// The object type this one inherits from. In Rust the parent becomes a
    /// field named `base`, since Rust has no struct inheritance.
    pub parent: Option<Rc<ObjectType>>,
    /// The fields declared by this type itself (not the inherited ones).
    pub fields: Vec<Field>,
}

/// Object types are equal when they're the same declaration.
impl PartialEq for ObjectType {
    fn eq(&self, other: &ObjectType) -> bool {
        self.rust_name == other.rust_name
    }
}

impl ObjectType {
    /// Find a field, own or inherited. Returns the Rust path to it from the
    /// object (`["base", "x"]` for a field inherited from the parent) and its
    /// type.
    pub fn field_path(&self, name: &str) -> Option<(Vec<String>, Type)> {
        if let Some(f) = self.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name)) {
            return Some((vec![f.rust_name.clone()], f.ty.clone()));
        }
        let (mut path, ty) = self.parent.as_ref()?.field_path(name)?;
        path.insert(0, "base".to_string());
        Some((path, ty))
    }

    /// This type and its ancestors, nearest first, each with the path to it
    /// (`[]`, `["base"]`, `["base", "base"]`, ...).
    pub fn lineage(self: &Rc<ObjectType>) -> Vec<(Rc<ObjectType>, Vec<String>)> {
        let mut result = Vec::new();
        let mut current = Some(self.clone());
        let mut path = Vec::new();
        while let Some(o) = current {
            result.push((o.clone(), path.clone()));
            path.push("base".to_string());
            current = o.parent.clone();
        }
        result
    }

    pub fn is_copy(&self) -> bool {
        self.fields.iter().all(|f| f.ty.is_copy()) && self.parent.as_ref().is_none_or(|p| p.is_copy())
    }
}

impl RecordType {
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name))
    }
}

impl Type {
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Int(_) | Type::IntLit(_))
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || *self == Type::Real
    }

    pub fn is_stringish(&self) -> bool {
        matches!(self, Type::Str { .. } | Type::StrLit(_) | Type::Char)
    }

    /// Pascal "ordinal" types: integers, chars, booleans (and later enums
    /// and subranges). Only these can index arrays, drive `for` loops and
    /// appear in `case` labels.
    pub fn is_ordinal(&self) -> bool {
        self.is_integer() || matches!(self, Type::Char | Type::Bool | Type::StrLit(1) | Type::Enum(_))
    }

    /// Replace "literal" types with the concrete type a variable of that type
    /// would have, e.g. for `var x := 5`-like inference of constants.
    pub fn concrete(&self) -> Type {
        match self {
            Type::IntLit(v) => Type::Int(IntKind::for_literal(*v)),
            Type::StrLit(1) => Type::Char,
            Type::StrLit(_) => Type::Str { cap: 255 },
            other => other.clone(),
        }
    }

    /// The Rust spelling of this type.
    pub fn rust(&self) -> String {
        match self {
            Type::Int(k) => k.rust().to_string(),
            Type::IntLit(v) => IntKind::for_literal(*v).rust().to_string(),
            Type::Real => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Char => "u8".to_string(),
            Type::Str { .. } | Type::StrLit(_) => "PString".to_string(),
            Type::Enum(e) => e.rust_name.clone(),
            Type::Record(r) => r.rust_name.clone(),
            Type::Object(o) => o.rust_name.clone(),
            Type::Text => "PText".to_string(),
            Type::File(elem) if **elem == Type::Untyped => "PBlockFile".to_string(),
            Type::File(elem) => format!("PFile<{}>", elem.rust()),
            Type::Pointer(p) => match &p.alias {
                Some(alias) => alias.clone(),
                None => format!("Ptr<{}>", p.target().rust()),
            },
            Type::Array(a) => match &a.alias {
                Some(alias) => alias.clone(),
                None => a.rust_unaliased(),
            },
            Type::TypeInfo => "&'static str".to_string(),
            Type::Untyped => "&mut dyn ByteView".to_string(),
            Type::Set(elem) => format!("PSet<{}>", elem.concrete().rust()),
            Type::Nil | Type::Void | Type::Error => "()".to_string(),
        }
    }

    /// Does the Rust type implement `Copy`? Decides whether codegen needs
    /// `.clone()` when Pascal copies a value.
    pub fn is_copy(&self) -> bool {
        match self {
            Type::Array(a) => a.elem.is_copy(),
            Type::Record(r) => r.fields.iter().all(|f| f.ty.is_copy()),
            Type::Object(o) => o.is_copy(),
            // Copying a pointer means cloning its `Rc` handle.
            Type::Pointer(_) => false,
            Type::Text | Type::File(_) => false,
            _ => true,
        }
    }

    /// Is a value of type `from` assignable to a variable of type `self`?
    /// This is Pascal's "assignment compatibility".
    pub fn accepts(&self, from: &Type) -> bool {
        match (self, from) {
            (_, Type::Error) | (Type::Error, _) => true,
            (Type::Int(_), t) if t.is_integer() => true,
            (Type::Real, t) if t.is_numeric() => true,
            (Type::Char, Type::Char | Type::StrLit(1)) => true,
            (Type::Str { .. }, t) if t.is_stringish() => true,
            (Type::Pointer(_), Type::Nil) => true,
            (Type::Set(a), Type::Set(b)) => **b == Type::Void || same_ordinal_kind(a, b),
            // An object may be assigned to (or passed as) one of its
            // ancestors: its ancestor part is copied.
            (Type::Object(to), Type::Object(from)) => from.lineage().iter().any(|(o, _)| o == to),
            // A pointer to an object may point to any object descended from it.
            (Type::Pointer(to), Type::Pointer(from)) if to != from => match (to.target(), from.target()) {
                (Type::Object(to), Type::Object(from)) => from.lineage().iter().any(|(o, _)| *o == to),
                _ => false,
            },
            (a, b) => a == b,
        }
    }
}

impl ArrayType {
    /// `PArray<T, LO, N>`, spelled out without the alias. A negative const
    /// generic argument needs braces in Rust: `PArray<i16, { -5 }, 11>`.
    pub fn rust_unaliased(&self) -> String {
        let lo = if self.lo < 0 { format!("{{ {} }}", self.lo) } else { self.lo.to_string() };
        format!("PArray<{}, {lo}, {}>", self.elem.rust(), self.count())
    }
}

impl fmt::Display for Type {
    /// Pascal spelling, for error messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int(k) => write!(f, "{k:?}"),
            Type::IntLit(_) => write!(f, "Integer"),
            Type::Real => write!(f, "Real"),
            Type::Bool => write!(f, "Boolean"),
            Type::Char => write!(f, "Char"),
            Type::Str { cap: 255 } | Type::StrLit(_) => write!(f, "String"),
            Type::Str { cap } => write!(f, "String[{cap}]"),
            Type::Enum(e) => write!(f, "{}", e.rust_name),
            Type::Record(r) => write!(f, "{}", r.rust_name),
            Type::Object(o) => write!(f, "{}", o.rust_name),
            Type::Pointer(p) => write!(f, "^{}", p.target_name),
            Type::Text => write!(f, "Text"),
            Type::File(elem) if **elem == Type::Untyped => write!(f, "file"),
            Type::File(elem) => write!(f, "file of {elem}"),
            Type::Array(a) => write!(f, "array[{}..{}] of {}", a.lo, a.hi, a.elem),
            Type::Nil | Type::TypeInfo => write!(f, "Pointer"),
            Type::Untyped => write!(f, "untyped"),
            Type::Set(elem) if **elem == Type::Void => write!(f, "empty set"),
            Type::Set(elem) => write!(f, "set of {elem}"),
            Type::Void => write!(f, "no value"),
            Type::Error => write!(f, "<error>"),
        }
    }
}

/// The size in bytes Turbo Pascal gives a type, for `SizeOf`: `Integer` is
/// 2, `Real` 6, `string[n]` n+1, a record the sum of its fields (TP doesn't
/// pad). `None` for a type without a fixed size.
pub fn tp_size(ty: &Type) -> Option<i64> {
    Some(match ty {
        Type::Int(IntKind::ShortInt | IntKind::Byte) | Type::Char | Type::Bool => 1,
        Type::Int(IntKind::Integer | IntKind::Word) => 2,
        Type::Int(IntKind::LongInt) => 4,
        // `Double`, `Single` and `Extended` are all `Real` here, so they
        // get `Real`'s 6 bytes too.
        Type::Real => 6,
        Type::Enum(e) => {
            if e.variants.len() <= 256 {
                1
            } else {
                2
            }
        }
        Type::Str { cap } => i64::from(*cap) + 1,
        Type::Array(a) => a.count() * tp_size(&a.elem)?,
        Type::Record(r) => r.fields.iter().map(|f| tp_size(&f.ty)).sum::<Option<i64>>()?,
        Type::Object(o) => {
            let own = o.fields.iter().map(|f| tp_size(&f.ty)).sum::<Option<i64>>()?;
            own + o.parent.as_ref().map_or(Some(0), |p| tp_size(&Type::Object(p.clone())))?
        }
        Type::Pointer(_) => 4,
        // A set has a bit per value, up to 256 values (32 bytes).
        Type::Set(elem) => match &**elem {
            Type::Bool => 1,
            Type::Enum(e) => (e.variants.len() as i64 + 7) / 8,
            _ => 32,
        },
        Type::Text => 256,
        Type::File(_) => 128,
        _ => return None,
    })
}

/// Can values of these two ordinal types mix, as set elements? Integers go
/// with integers, characters with characters, an enum with itself.
pub fn same_ordinal_kind(a: &Type, b: &Type) -> bool {
    match (a.concrete(), b.concrete()) {
        (x, y) if x.is_integer() && y.is_integer() => true,
        (x, y) => x == y,
    }
}

#[cfg(test)]
mod tests {
    use super::IntKind::*;
    use super::*;

    #[test]
    fn integer_promotion_follows_turbo_pascal() {
        assert_eq!(IntKind::common(Integer, Integer), Integer);
        assert_eq!(IntKind::common(Byte, Byte), Integer);
        assert_eq!(IntKind::common(Byte, ShortInt), Integer);
        assert_eq!(IntKind::common(Integer, LongInt), LongInt);
        assert_eq!(IntKind::common(Integer, Word), LongInt);
        assert_eq!(IntKind::common(Word, Byte), Word);
    }
}
