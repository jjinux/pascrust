//! Expressions, type conversions and literals.
//!
//! Every function here returns the Rust code *and its precedence*, so that
//! callers can add parentheses only where Rust needs them. Pascal and Rust
//! rank operators differently (in Pascal `and` binds like `*`; in Rust `&&`
//! binds looser than `==`), so the parse tree has to be re-parenthesized for
//! Rust rather than copied.

use super::Codegen;
use crate::ast::*;
use crate::sema::{Access, ConstVal, IntKind, SymKind, Type, numeric_common};

/// Rust operator precedence, loosest first. Only the relative order matters.
pub(super) mod prec {
    pub const OR: u8 = 1; // ||
    pub const AND: u8 = 2; // &&
    pub const CMP: u8 = 3; // == != < > <= >=
    pub const BIT_OR: u8 = 4; // |
    pub const BIT_XOR: u8 = 5; // ^
    pub const BIT_AND: u8 = 6; // &
    pub const SHIFT: u8 = 7; // << >>
    pub const ADD: u8 = 8; // + -
    pub const MUL: u8 = 9; // * / %
    pub const AS: u8 = 10; // `x as T`
    pub const UNARY: u8 = 11; // -x !x *x
    pub const POSTFIX: u8 = 12; // calls, methods, fields, indexing
    pub const ATOM: u8 = 13; // names and literals
}

/// Code plus its precedence.
pub(super) type Code = (String, u8);

/// Wrap `code` in parentheses if it binds looser than `min`.
pub(super) fn paren(code: Code, min: u8) -> String {
    if code.1 < min {
        format!("({})", code.0)
    } else {
        code.0
    }
}

impl<'a> Codegen<'a> {
    /// An expression in its natural Rust type.
    pub(super) fn expr(&mut self, e: &Expr) -> Code {
        match &e.kind {
            ExprKind::Int(n) => (n.to_string(), prec::ATOM),
            ExprKind::Real(x) => (real_literal(*x), prec::ATOM),
            ExprKind::Str(bytes) => (str_literal(bytes), prec::ATOM),
            ExprKind::Nil => ("Ptr::nil()".to_string(), prec::POSTFIX),
            ExprKind::Name(_) => self.name(e),
            ExprKind::Call { callee, args } => (self.call(callee, args), prec::POSTFIX),
            ExprKind::Unary { op, operand } => {
                let ty = self.a.type_of(e.id).clone();
                let inner = paren(self.coerce(operand, &ty), prec::UNARY);
                match op {
                    UnaryOp::Neg => (format!("-{inner}"), prec::UNARY),
                    UnaryOp::Not => (format!("!{inner}"), prec::UNARY),
                    UnaryOp::Plus => self.coerce(operand, &ty),
                }
            }
            ExprKind::Binary { op, lhs, rhs } => self.binary(e, *op, lhs, rhs),
            ExprKind::Format { value, .. } => self.expr(value),
            _ => unreachable!("sema rejects this expression"),
        }
    }

    fn name(&mut self, e: &Expr) -> Code {
        let sym = self.a.refs[&e.id];
        let symbol = self.a.sym(sym);
        match &symbol.kind {
            SymKind::Var(_) => match self.a.access(self.proc, sym) {
                Access::Direct => (symbol.rust_name.clone(), prec::ATOM),
                // A reference must be dereferenced to get at the value.
                Access::Reference => (format!("*{}", symbol.rust_name), prec::UNARY),
            },
            SymKind::Const { .. } => (symbol.rust_name.clone(), prec::ATOM),
            SymKind::Proc(_) | SymKind::Builtin(_) => (self.call(e, &[]), prec::POSTFIX),
            SymKind::TypeName(_) => unreachable!("sema rejects types used as values"),
        }
    }

    /// An expression converted to type `to`. Pascal converts between number
    /// types silently; Rust insists on explicit conversions, so this is where
    /// the `i32::from(x)` and `x as i16` in the generated code come from.
    pub(super) fn coerce(&mut self, e: &Expr, to: &Type) -> Code {
        let from = self.a.type_of(e.id).clone();
        // A one-character string literal used as a `Char` becomes a byte
        // literal: `'A'` → `b'A'`.
        if *to == Type::Char
            && let ExprKind::Str(bytes) = &e.kind
            && bytes.len() == 1
        {
            return (char_literal(bytes[0]), prec::ATOM);
        }
        if let (Type::Real, Type::IntLit(_)) = (to, &from)
            && let ExprKind::Int(n) = e.kind
        {
            return (format!("{n}.0"), prec::ATOM);
        }
        let code = self.expr(e);
        convert(code, &from, to)
    }

    /// An expression used as the receiver of a method call (`x.abs()`):
    /// parenthesized if needed, and with integer literals given an explicit
    /// type, because Rust can't call methods on a literal of unknown type.
    pub(super) fn postfix(&mut self, e: &Expr, ty: &Type) -> String {
        // Method calls dereference automatically, so a reference variable
        // can be used as is: `x.abs()` rather than `(*x).abs()`.
        if let ExprKind::Name(_) = e.kind
            && let Some(&sym) = self.a.refs.get(&e.id)
            && let SymKind::Var(_) = self.a.sym(sym).kind
            && *self.a.type_of(e.id) == *ty
        {
            return self.a.sym(sym).rust_name.clone();
        }
        match (&e.kind, ty) {
            (ExprKind::Int(n), Type::Int(k)) => format!("{n}_{}", k.rust()),
            (ExprKind::Int(n), Type::Real) => format!("{n}.0_f64"),
            (ExprKind::Real(x), _) => format!("{}_f64", real_literal(*x)),
            _ => paren(self.coerce(e, ty), prec::POSTFIX),
        }
    }

    fn binary(&mut self, e: &Expr, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> Code {
        use BinaryOp::*;
        let ty = self.a.type_of(e.id).clone();
        let (lt, rt) = (
            self.a.type_of(lhs.id).clone(),
            self.a.type_of(rhs.id).clone(),
        );
        let (rust_op, p) = match op {
            Add => ("+", prec::ADD),
            Sub => ("-", prec::ADD),
            Mul => ("*", prec::MUL),
            RealDiv | Div => ("/", prec::MUL),
            Mod => ("%", prec::MUL),
            And if ty == Type::Bool => ("&&", prec::AND),
            Or if ty == Type::Bool => ("||", prec::OR),
            And => ("&", prec::BIT_AND),
            Or => ("|", prec::BIT_OR),
            Xor => ("^", prec::BIT_XOR),
            Shl => ("<<", prec::SHIFT),
            Shr => (">>", prec::SHIFT),
            Eq => ("==", prec::CMP),
            NotEq => ("!=", prec::CMP),
            Lt => ("<", prec::CMP),
            LtEq => ("<=", prec::CMP),
            Gt => (">", prec::CMP),
            GtEq => (">=", prec::CMP),
            In => unreachable!("sema rejects `in` without sets"),
        };
        // The type both operands are converted to before the operation.
        let operand_ty = match op {
            Eq | NotEq | Lt | LtEq | Gt | GtEq => comparison_type(&lt, &rt),
            Shl | Shr => ty.clone(),
            _ => ty.clone(),
        };
        let left = self.coerce(lhs, &operand_ty);
        let right = match (op, &operand_ty, &rt) {
            // `PString` implements `+` and `==` for string literals and
            // chars directly, so the right operand can stay as it is.
            (
                Add | Eq | NotEq | Lt | LtEq | Gt | GtEq,
                Type::Str { .. },
                Type::StrLit(_) | Type::Char,
            ) => self.expr(rhs),
            (Shl | Shr, _, _) => self.expr(rhs),
            _ => self.coerce(rhs, &operand_ty),
        };
        // `x as i16 < y` would parse `<` as the start of generic arguments,
        // so a cast on the left of `<` or `<<` always gets parentheses.
        let left_min = if left.1 == prec::AS && matches!(op, Lt | Shl) {
            prec::UNARY
        } else {
            p
        };
        // Comparisons don't chain in Rust, so both sides need higher
        // precedence; arithmetic is left-associative, so only the right side
        // does.
        let left_min = if p == prec::CMP { p + 1 } else { left_min };
        let code = format!(
            "{} {rust_op} {}",
            paren(left, left_min),
            paren(right, p + 1)
        );
        (code, p)
    }

    /// The Rust place expression for an assignment target.
    pub(super) fn place(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Name(_) => self.var_place(self.a.refs[&e.id]),
            _ => unreachable!("sema rejects other assignment targets"),
        }
    }

    /// A mutable reference to a place, for a `var` argument: `&mut x`, or
    /// just `x` if we already hold `x` as a reference (Rust "reborrows" it).
    pub(super) fn place_mut(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Name(_) => {
                let sym = self.a.refs[&e.id];
                let name = self.a.sym(sym).rust_name.clone();
                match self.a.access(self.proc, sym) {
                    Access::Direct => format!("&mut {name}"),
                    Access::Reference => name,
                }
            }
            _ => format!("&mut {}", self.place(e)),
        }
    }

    /// A call to a user procedure, a builtin or a type cast.
    pub(super) fn call(&mut self, callee: &Expr, args: &[Expr]) -> String {
        let sym = self.a.refs[&callee.id];
        match &self.a.sym(sym).kind {
            SymKind::Proc(p) => self.user_call(*p, args),
            SymKind::Builtin(b) => self.builtin(*b, args),
            SymKind::TypeName(ty) => {
                let ty = ty.clone();
                let from = self.a.type_of(args[0].id).clone();
                let code = self.coerce(&args[0], &from.concrete());
                cast(code, &from.concrete(), &ty).0
            }
            SymKind::Var(_) | SymKind::Const { .. } => self.expr(callee).0,
        }
    }

    fn user_call(&mut self, p: usize, args: &[Expr]) -> String {
        let info = &self.a.procs[p];
        let mut parts = Vec::new();
        // Captured variables first, in the callee's order.
        for cap in &info.captures {
            let name = self.a.sym(cap.sym).rust_name.clone();
            let access = self.a.access(self.proc, cap.sym);
            parts.push(match (cap.mode, access) {
                (crate::sema::CaptureMode::RefMut, Access::Direct) => format!("&mut {name}"),
                (crate::sema::CaptureMode::Ref, Access::Direct) => format!("&{name}"),
                (crate::sema::CaptureMode::Value, Access::Reference) => format!("*{name}"),
                // We hold it by reference and pass a reference on, or we hold
                // it by value and pass it by value.
                _ => name,
            });
        }
        let params = info.params.clone();
        for (param, arg) in params.iter().zip(args) {
            let var = self.a.var(*param);
            let ty = var.ty.clone();
            if var.role == crate::sema::VarRole::Param(ParamMode::Var) {
                parts.push(self.place_mut(arg));
            } else {
                parts.push(self.coerce(arg, &ty).0);
            }
        }
        format!("{}({})", self.a.procs[p].rust_name, parts.join(", "))
    }

    /// The Rust literal for a constant of type `ty`.
    pub(super) fn const_value(&self, value: &ConstVal, ty: &Type) -> String {
        match value {
            ConstVal::Int(n) => n.to_string(),
            ConstVal::Real(x) => real_literal(*x),
            ConstVal::Bool(b) => b.to_string(),
            ConstVal::Str(bytes) if *ty == Type::Char && bytes.len() == 1 => char_literal(bytes[0]),
            ConstVal::Str(bytes) => str_literal(bytes),
        }
    }
}

/// The type in which two values are compared.
fn comparison_type(l: &Type, r: &Type) -> Type {
    match (l, r) {
        _ if l.is_numeric() && r.is_numeric() => numeric_common(l, r).unwrap_or(Type::Error),
        (Type::Char | Type::StrLit(1), Type::Char | Type::StrLit(1)) => Type::Char,
        _ if l.is_stringish() && r.is_stringish() => Type::Str { cap: 255 },
        _ => l.concrete(),
    }
}

/// Convert code of type `from` to type `to` (both Pascal types).
pub(super) fn convert(code: Code, from: &Type, to: &Type) -> Code {
    match (from, to) {
        (a, b) if a == b => code,
        (Type::IntLit(_), Type::Int(_)) => code,
        (Type::Int(_) | Type::IntLit(_) | Type::Char | Type::Bool, Type::Int(_) | Type::Char) => {
            cast(code, &from.concrete(), to)
        }
        (Type::Int(_) | Type::IntLit(_), Type::Real) => {
            (format!("f64::from({})", code.0), prec::POSTFIX)
        }
        (Type::StrLit(_) | Type::Char, Type::Str { .. }) => {
            (format!("pstr({})", code.0), prec::POSTFIX)
        }
        _ => code,
    }
}

/// An explicit conversion between ordinal types. Widening conversions that
/// can't lose information use `From` (`i32::from(x)`), which Rust
/// programmers prefer because it can never silently truncate; narrowing
/// ones use `as`, which truncates exactly like Turbo Pascal does.
pub(super) fn cast(code: Code, from: &Type, to: &Type) -> Code {
    if from == to {
        return code;
    }
    let lossless = match (from, to) {
        (Type::Int(a), Type::Int(b)) => {
            let (alo, ahi) = a.range();
            let (blo, bhi) = b.range();
            blo <= alo && ahi <= bhi
        }
        (Type::Char | Type::Bool, Type::Int(b)) => b.contains(255) && *b != IntKind::ShortInt,
        (Type::Bool, Type::Char) => true,
        _ => false,
    };
    let rust = to.rust();
    if lossless {
        (format!("{rust}::from({})", code.0), prec::POSTFIX)
    } else {
        (format!("{} as {rust}", paren(code, prec::AS)), prec::AS)
    }
}

/// A Rust float literal: always with a `.` or exponent, so that `5.0`
/// doesn't come out as the integer `5`.
pub(super) fn real_literal(x: f64) -> String {
    // `{:?}` on an f64 prints the shortest text that reads back as the same
    // number, and always includes `.0` for whole numbers.
    format!("{x:?}")
}

/// A Rust string literal for Pascal string bytes. Plain ASCII becomes a
/// normal `"..."` string; anything else (code page 437 box drawing, for
/// instance) becomes a byte string `b"..."`, since Rust strings must be UTF-8.
pub(super) fn str_literal(bytes: &[u8]) -> String {
    let prefix = if bytes.iter().all(|b| b.is_ascii()) {
        ""
    } else {
        "b"
    };
    let mut s = format!("{prefix}\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    s
}

/// A Rust byte literal for a Pascal `Char`: `b'A'`, `b'\''`, `b'\x07'`.
pub(super) fn char_literal(b: u8) -> String {
    match b {
        b'\'' => "b'\\''".to_string(),
        b'\\' => "b'\\\\'".to_string(),
        b'\n' => "b'\\n'".to_string(),
        b'\r' => "b'\\r'".to_string(),
        b'\t' => "b'\\t'".to_string(),
        0x20..=0x7e => format!("b'{}'", b as char),
        _ => format!("b'\\x{b:02x}'"),
    }
}
