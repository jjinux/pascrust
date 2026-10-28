//! Pascal types as sema understands them, and how each maps to Rust.

use std::fmt;

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
        if IntKind::Integer.contains(value) {
            IntKind::Integer
        } else {
            IntKind::LongInt
        }
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
    /// The type of `nil`.
    Nil,
    /// The "type" of a procedure call, which has no value.
    Void,
    /// Produced after a type error so that one mistake doesn't cascade into
    /// many error messages.
    Error,
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
        self.is_integer() || matches!(self, Type::Char | Type::Bool | Type::StrLit(1))
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
            Type::Nil | Type::Void | Type::Error => "()".to_string(),
        }
    }

    /// Does the Rust type implement `Copy`? Decides whether codegen needs
    /// `.clone()` when Pascal copies a value.
    pub fn is_copy(&self) -> bool {
        true
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
            (a, b) => a == b,
        }
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
            Type::Nil => write!(f, "Pointer"),
            Type::Void => write!(f, "no value"),
            Type::Error => write!(f, "<error>"),
        }
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
