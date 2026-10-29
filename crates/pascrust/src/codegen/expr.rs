//! Expressions, type conversions and literals.
//!
//! Every function here returns the Rust code *and its precedence*, so that
//! callers can add parentheses only where Rust needs them. Pascal and Rust
//! rank operators differently (in Pascal `and` binds like `*`; in Rust `&&`
//! binds looser than `==`), so the parse tree has to be re-parenthesized for
//! Rust rather than copied.

use super::Codegen;
use crate::ast::*;
use crate::sema::{Access, Builtin, ConstVal, IntKind, SymId, SymKind, Type, numeric_common};

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
    if code.1 < min { format!("({})", code.0) } else { code.0 }
}

impl<'a> Codegen<'a> {
    /// An expression in its natural Rust type.
    pub(super) fn expr(&mut self, e: &Expr) -> Code {
        if let Some(name) = self.hoisted.get(&e.id) {
            return (name.clone(), prec::ATOM);
        }
        // `p^.n + F`: the read through `p` would keep the record borrowed
        // while `F` runs, and `F` may change it (through another pointer).
        // So `p^.n` is read into a `let` first.
        if let ExprKind::Binary { lhs, rhs, .. } = &e.kind
            && !self.hoisted.contains_key(&lhs.id)
            && reads_pointer(lhs)
            && self.has_call(rhs)
        {
            let code = self.expr(lhs).0;
            let name = self.temp_name(&super::value_temp_base(lhs));
            self.pending.push(format!("let {name} = {code};"));
            self.hoisted.insert(lhs.id, name);
        }
        // `items[Count]` in a method is `self.items[self.count()]`, which
        // borrows `self.items` while `count` needs all of `self`. An index
        // whose call can reach the array is worked out first, into a `let`.
        if let ExprKind::Index(base, indexes) = &e.kind {
            let root = super::root_expr(base);
            for index in indexes {
                if !self.hoisted.contains_key(&index.id) && self.call_reaches(index, root) {
                    let code = self.expr(index).0;
                    let name = self.temp_name("index");
                    self.pending.push(format!("let {name} = {code};"));
                    self.hoisted.insert(index.id, name);
                }
            }
        }
        if let Some(value) = self.a.folded.get(&e.id) {
            let ty = self.a.type_of(e.id).clone();
            // A folded `Hi(-1)` is a Byte: `255_u8`, so it converts like one.
            if let (ConstVal::Int(n), Type::Int(kind)) = (value, &ty) {
                let precedence = if *n < 0 { prec::UNARY } else { prec::ATOM };
                return (format!("{n}_{}", kind.rust()), precedence);
            }
            return (self.const_value(value, &ty), prec::ATOM);
        }
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
                    // `--x` reads like C's decrement, and rustc warns about it.
                    UnaryOp::Neg if inner.starts_with('-') => (format!("-({inner})"), prec::UNARY),
                    UnaryOp::Neg => (format!("-{inner}"), prec::UNARY),
                    UnaryOp::Not => (format!("!{inner}"), prec::UNARY),
                    UnaryOp::Plus => self.coerce(operand, &ty),
                }
            }
            ExprKind::Binary { op, lhs, rhs } => self.binary(e, *op, lhs, rhs),
            ExprKind::Format { value, .. } => self.expr(value),
            ExprKind::Field(..) if self.a.method_calls.contains_key(&e.id) => (self.call(e, &[]), prec::POSTFIX),
            // A whole record through a pointer: copy it out of the cell.
            ExprKind::Deref(_) => {
                let code = self.selector(e);
                if self.a.type_of(e.id).is_copy() {
                    (format!("*{code}"), prec::UNARY)
                } else {
                    (format!("{code}.clone()"), prec::POSTFIX)
                }
            }
            ExprKind::Field(..) | ExprKind::Index(..) => (self.selector(e), prec::POSTFIX),
            ExprKind::Set(elems) => (self.set_literal(e, elems), prec::POSTFIX),
            _ => unreachable!("sema rejects this expression"),
        }
    }

    /// `[1, 3..5, x]` → `pset![1, 3..=5, x]`, each element as the set's
    /// element type (`'a'` → `b'a'`).
    fn set_literal(&mut self, e: &Expr, elems: &[SetElem]) -> String {
        let Type::Set(elem_ty) = self.a.type_of(e.id).clone() else { unreachable!("sema typed the set") };
        if elems.is_empty() {
            return "PSet::new()".to_string();
        }
        let items: Vec<String> = elems.iter().map(|elem| self.set_item(elem, &elem_ty, false)).collect();
        format!("pset![{}]", items.join(", "))
    }

    /// One element of a set literal, or, with `pattern`, of the `matches!`
    /// pattern that tests `x in [...]` against constants.
    fn set_item(&mut self, elem: &SetElem, elem_ty: &Type, pattern: bool) -> String {
        match elem {
            SetElem::Single(x) => self.coerce(x, elem_ty).0,
            SetElem::Range(lo, hi) => {
                // Rust range patterns are for numbers and chars: in
                // `d in [Sat..Sun]` an enum's (or Boolean's) values are
                // listed instead. (A set value can take a range of anything
                // ordinal: `pset![Day::Sat..=Day::Sun]`.)
                if pattern
                    && matches!(elem_ty, Type::Enum(_) | Type::Bool)
                    && let (Some(lo), Some(hi)) = (self.const_ordinal(lo), self.const_ordinal(hi))
                {
                    return self.ord_range_pattern(elem_ty, lo, hi);
                }
                let (lo, hi) = (self.coerce(lo, elem_ty), self.coerce(hi, elem_ty));
                // Any binary operator binds tighter than `..=`.
                format!("{}..={}", paren(lo, prec::OR), paren(hi, prec::OR))
            }
        }
    }

    /// Set operators: `+` → `|`, `*` → `&`, `-` stays, `<=`/`>=` are
    /// `is_subset`/`is_superset`, and `x in s` is `s.contains(x)`, or, for a
    /// list of constants, `matches!(x, 1 | 3..=5)`.
    fn set_binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> Code {
        use BinaryOp::*;
        if op == In {
            let Type::Set(elem_ty) = self.a.type_of(rhs.id).clone() else { unreachable!("sema checked `in`") };
            let x = self.coerce(lhs, &elem_ty);
            if let ExprKind::Set(elems) = &rhs.kind
                && elems.iter().all(|elem| self.constant_item(elem))
            {
                // A backwards range (`[Fri..Mon]`) is empty, as in TP; Rust
                // rejects it as a pattern.
                let elems: Vec<&SetElem> = elems
                    .iter()
                    .filter(|elem| match elem {
                        SetElem::Range(lo, hi) => {
                            !matches!((self.const_ordinal(lo), self.const_ordinal(hi)), (Some(l), Some(h)) if l > h)
                        }
                        SetElem::Single(_) => true,
                    })
                    .collect();
                if elems.is_empty() {
                    return ("false".to_string(), prec::POSTFIX);
                }
                let patterns: Vec<String> = elems.into_iter().map(|elem| self.set_item(elem, &elem_ty, true)).collect();
                return (format!("matches!({}, {})", x.0, patterns.join(" | ")), prec::POSTFIX);
            }
            // A literal tested at once has nothing else to say what it holds.
            let set = match &rhs.kind {
                ExprKind::Set(elems) if !elems.is_empty() => {
                    let items: Vec<String> = elems.iter().map(|elem| self.set_item(elem, &elem_ty, false)).collect();
                    (format!("pset![set of {}; {}]", elem_ty.rust(), items.join(", ")), prec::ATOM)
                }
                _ => self.expr(rhs),
            };
            // `contains` takes any ordinal, so an integer is passed as it
            // is: `300 in s` is false for a set of bytes, rather than
            // testing 300 cut down to a byte (44).
            let x = if self.a.type_of(lhs.id).is_integer() { self.expr(lhs) } else { x };
            return (format!("{}.contains({})", paren(set, prec::POSTFIX), x.0), prec::POSTFIX);
        }
        let (l, r) = (self.expr(lhs), self.expr(rhs));
        let (rust_op, p) = match op {
            Add => ("|", prec::BIT_OR),
            Mul => ("&", prec::BIT_AND),
            Sub => ("-", prec::ADD),
            Eq => ("==", prec::CMP),
            NotEq => ("!=", prec::CMP),
            LtEq | GtEq => {
                let method = if op == LtEq { "is_subset" } else { "is_superset" };
                // `&` borrows the right-hand set; a literal needs no `&`
                // around its parentheses, `&pset![...]` is fine.
                return (format!("{}.{method}(&{})", paren(l, prec::POSTFIX), paren(r, prec::UNARY)), prec::POSTFIX);
            }
            _ => unreachable!("sema checked the set operator"),
        };
        let left_min = if p == prec::CMP { p + 1 } else { p };
        (format!("{} {rust_op} {}", paren(l, left_min), paren(r, p + 1)), p)
    }

    /// The ordinal of a literal or a named constant (`5`, `'a'`, an enum
    /// value, `True`).
    fn const_ordinal(&self, e: &Expr) -> Option<i64> {
        let value = match &e.kind {
            ExprKind::Int(n) => return Some(*n),
            ExprKind::Str(bytes) if bytes.len() == 1 => return Some(i64::from(bytes[0])),
            _ => match &self.a.sym(*self.a.refs.get(&e.id)?).kind {
                SymKind::Const { value, .. } => value,
                _ => return None,
            },
        };
        match value {
            ConstVal::Int(n) => Some(*n),
            ConstVal::Bool(b) => Some(*b as i64),
            ConstVal::Str(bytes) if bytes.len() == 1 => Some(i64::from(bytes[0])),
            _ => None,
        }
    }

    /// Can this set element be a `match` pattern: a literal, a named
    /// constant or an enum value?
    fn constant_item(&self, elem: &SetElem) -> bool {
        let constant = |e: &Expr| match &e.kind {
            ExprKind::Int(_) | ExprKind::Str(_) => true,
            ExprKind::Name(_) => {
                self.a.refs.get(&e.id).is_some_and(|&s| matches!(self.a.sym(s).kind, SymKind::Const { .. }))
            }
            _ => false,
        };
        match elem {
            SetElem::Single(e) => constant(e),
            SetElem::Range(lo, hi) => constant(lo) && constant(hi),
        }
    }

    fn name(&mut self, e: &Expr) -> Code {
        // Inside a method, a bare field name means `self.field`, and a bare
        // method name is a call on `self`.
        if let Some((self_sym, path)) = self.a.self_fields.get(&e.id) {
            return (format!("{}.{}", self.self_place(*self_sym), path.join(".")), prec::POSTFIX);
        }
        if self.a.method_calls.contains_key(&e.id) {
            return (self.call(e, &[]), prec::POSTFIX);
        }
        let sym = self.a.refs[&e.id];
        let symbol = self.a.sym(sym);
        match &symbol.kind {
            SymKind::Var(_) => match self.a.access(self.proc, sym) {
                Access::Direct => (self.a.var_name(self.proc, sym).to_string(), prec::ATOM),
                // A reference must be dereferenced to get at the value.
                Access::Reference => (format!("*{}", self.a.var_name(self.proc, sym)), prec::UNARY),
            },
            SymKind::Const { .. } => (symbol.rust_name.clone(), prec::ATOM),
            SymKind::RuntimeVar { get, .. } => (format!("{get}()"), prec::POSTFIX),
            // The printer: the runtime's `Lst`, as a file argument
            // (`fwriteln!(Lst, ...)`). `Input` and `Output` never get here.
            SymKind::StdText(_) => (symbol.rust_name.clone(), prec::ATOM),
            SymKind::Proc(_) | SymKind::Builtin(_) | SymKind::UnitRoutine(_) => (self.call(e, &[]), prec::POSTFIX),
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
        if let (Type::Str { .. }, ExprKind::Str(bytes)) = (to, &e.kind)
            && bytes.is_empty()
        {
            return ("PString::new()".to_string(), prec::POSTFIX);
        }
        // A descendant object where an ancestor is expected: its ancestor
        // part, `circle.base` (cloned unless it's `Copy`). TP copied its
        // table of virtual methods too, so the copy would still act as a
        // circle there; here it's the ancestor it now is.
        if let (Type::Object(to_object), Type::Object(from_object)) = (to, &from)
            && let Some(depth) = from_object.lineage().iter().position(|(o, _)| o == to_object)
            && depth > 0
        {
            let base = match &e.kind {
                ExprKind::Name(_) | ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::Deref(_) => {
                    self.selector_base(e)
                }
                _ => paren(self.expr(e), prec::POSTFIX),
            };
            let part = format!("{base}{}", ".base".repeat(depth));
            return if to.is_copy() { (part, prec::POSTFIX) } else { (format!("{part}.clone()"), prec::POSTFIX) };
        }
        let code = self.expr(e);
        // Pascal copies records and pointers on assignment. For types that
        // aren't `Copy` in Rust (anything holding a `Ptr`), that copy has to
        // be an explicit `.clone()`. A value that's already fresh (a
        // function result, a literal) needs no clone.
        if !to.is_copy() && is_place(e) && !matches!(e.kind, ExprKind::Deref(_)) {
            let receiver = match (&e.kind, self.a.refs.get(&e.id)) {
                (ExprKind::Name(_), Some(&sym))
                    if matches!(self.a.sym(sym).kind, SymKind::Var(_)) && !self.a.self_fields.contains_key(&e.id) =>
                {
                    self.a.var_name(self.proc, sym).to_string()
                }
                _ => paren(code, prec::POSTFIX),
            };
            return (format!("{receiver}.clone()"), prec::POSTFIX);
        }
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
            return self.a.var_name(self.proc, sym).to_string();
        }
        // Literals get a type suffix: `5_i16.abs()`, `(-2.5_f64).round()`.
        let literal = match (self.a.type_of(e.id), ty, &e.kind) {
            (Type::IntLit(n), Type::Int(k), _) => Some(format!("{n}_{}", k.rust())),
            (Type::IntLit(n), Type::Real, _) => Some(format!("{n}.0_f64")),
            (_, _, ExprKind::Real(x)) => Some(format!("{}_f64", real_literal(*x))),
            (_, _, ExprKind::Unary { op: UnaryOp::Neg, operand }) => match operand.kind {
                ExprKind::Real(x) => Some(format!("-{}_f64", real_literal(x))),
                _ => None,
            },
            _ => None,
        };
        match literal {
            Some(text) if text.starts_with('-') => format!("({text})"),
            Some(text) => text,
            None => paren(self.coerce(e, ty), prec::POSTFIX),
        }
    }

    fn binary(&mut self, e: &Expr, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> Code {
        use BinaryOp::*;
        let ty = self.a.type_of(e.id).clone();
        let (lt, rt) = (self.a.type_of(lhs.id).clone(), self.a.type_of(rhs.id).clone());
        if op == In || matches!(lt, Type::Set(_)) {
            return self.set_binary(op, lhs, rhs);
        }
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
        // `TypeOf(p^) = TypeOf(TCircle)` compares type names.
        if lt == Type::TypeInfo {
            let (l, r) = (self.expr(lhs).0, self.expr(rhs).0);
            return (format!("{l} {rust_op} {r}"), p);
        }
        // `p = nil` reads best as `p.is_nil()`.
        if matches!(op, Eq | NotEq) && (lt == Type::Nil || rt == Type::Nil) && lt != rt {
            let pointer = if lt == Type::Nil { rhs } else { lhs };
            // No `.clone()` needed just to look at the pointer.
            let receiver = match &pointer.kind {
                ExprKind::Name(_) | ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::Deref(_) => {
                    self.selector_base(pointer)
                }
                _ => paren(self.expr(pointer), prec::POSTFIX),
            };
            let code = format!("{receiver}.is_nil()");
            return if op == Eq { (code, prec::POSTFIX) } else { (format!("!{code}"), prec::UNARY) };
        }
        // The type both operands are converted to before the operation.
        let operand_ty = match op {
            Eq | NotEq | Lt | LtEq | Gt | GtEq => comparison_type(&lt, &rt),
            _ => ty.clone(),
        };
        // Comparing two pointers only looks at them, so neither side needs
        // the `.clone()` that `coerce` gives a pointer it copies.
        let compare_pointers = matches!(op, Eq | NotEq) && matches!(operand_ty, Type::Pointer(_));
        let left = if compare_pointers { self.expr(lhs) } else { self.coerce(lhs, &operand_ty) };
        // The right side of `and`/`or` only runs if it has to: in
        // `(p <> nil) and (Show(p^.value) > 0)`, `p^.value` mustn't be read
        // when `p` is nil. So the `let`s it needs go inside it, in a block,
        // rather than before the whole statement.
        let short_circuit = matches!(op, And | Or) && ty == Type::Bool;
        let mark = self.pending.len();
        let right = match (op, &operand_ty, &rt) {
            _ if compare_pointers => self.expr(rhs),
            // `PString` implements `+` and `==` for string literals and
            // chars directly, so the right operand can stay as it is.
            (Add | Eq | NotEq | Lt | LtEq | Gt | GtEq, Type::Str { .. }, Type::StrLit(_) | Type::Char) => {
                self.expr(rhs)
            }
            (Shl | Shr, _, _) => self.expr(rhs),
            _ => self.coerce(rhs, &operand_ty),
        };
        let right = if short_circuit && self.pending.len() > mark {
            let (comments, lets): (Vec<String>, Vec<String>) =
                self.pending.split_off(mark).into_iter().partition(|line| line.starts_with("//"));
            self.pending.extend(comments);
            (format!("{{ {} {} }}", lets.join(" "), right.0), prec::ATOM)
        } else {
            right
        };
        if let (Shl | Shr, Type::Int(kind)) = (op, operand_ty.concrete()) {
            return self.shift(op, kind, (left, &lt), (right, &rt));
        }
        // `x as i16 < y` would parse `<` as the start of generic arguments,
        // so a cast on the left of `<` or `<<` always gets parentheses.
        let left_min = if left.1 == prec::AS && op == Lt { prec::UNARY } else { p };
        // Comparisons don't chain in Rust, so both sides need higher
        // precedence; arithmetic is left-associative, so only the right side
        // does.
        let left_min = if p == prec::CMP { p + 1 } else { left_min };
        let code = format!("{} {rust_op} {}", paren(left, left_min), paren(right, p + 1));
        (code, p)
    }

    /// `x shl n` or `x shr n` on an integer of type `kind`.
    fn shift(&mut self, op: BinaryOp, kind: IntKind, left: (Code, &Type), right: (Code, &Type)) -> Code {
        let ((left, lt), (right, rt)) = (left, right);
        let width = match kind {
            IntKind::ShortInt | IntKind::Byte => 8,
            IntKind::Integer | IntKind::Word => 16,
            IntKind::LongInt => 32,
        };
        // Turbo Pascal's `shr` is a logical shift: it fills with zeros, so
        // `-300 shr 4` is 4077. Rust's `>>` on a signed integer copies the
        // sign bit instead (-19), so shift the same bits as unsigned.
        // (A literal that isn't negative shifts the same either way.)
        let unsigned = match kind {
            _ if op == BinaryOp::Shl || matches!(lt, Type::IntLit(n) if *n >= 0) => None,
            IntKind::ShortInt => Some("u8"),
            IntKind::Integer => Some("u16"),
            IntKind::LongInt => Some("u32"),
            IntKind::Byte | IntKind::Word => None,
        };
        // A literal needs its type spelled out (`-300_i16`) to be cast or to
        // have a method called on it.
        let small_count = matches!(rt, Type::IntLit(n) if (0..width).contains(n));
        let left = if matches!(lt, Type::IntLit(_)) && (unsigned.is_some() || !small_count) {
            (format!("{}_{}", left.0, kind.rust()), prec::POSTFIX)
        } else {
            left
        };
        let left = match unsigned {
            Some(unsigned) => (format!("{} as {unsigned}", paren(left, prec::AS)), prec::AS),
            None => left,
        };
        let code = if small_count {
            let rust_op = if op == BinaryOp::Shl { "<<" } else { ">>" };
            // `x as i16 << y` would parse `<` as the start of generic
            // arguments, so a cast on the left of `<<` gets parentheses.
            let left_min = if left.1 == prec::AS { prec::UNARY } else { prec::SHIFT };
            (format!("{} {rust_op} {}", paren(left, left_min), paren(right, prec::SHIFT + 1)), prec::SHIFT)
        } else {
            // Shifting by the type's width or more shifts every bit out in
            // Turbo Pascal (`x shl 16` is 0), but Rust's `<<` rejects it (or
            // panics, for a count it can't see at compile time).
            // `checked_shl` gives `None` instead.
            if self.first_time("checked_shift") {
                self.pending
                    .push("// `checked_shl`/`checked_shr`: shifting by the integer's width or more gives".to_string());
                self.pending.push("// 0, as in Turbo Pascal, where Rust's `<<` and `>>` would panic.".to_string());
            }
            let method = if op == BinaryOp::Shl { "checked_shl" } else { "checked_shr" };
            let count = match rt {
                Type::IntLit(_) => right.0,
                _ => format!("{} as u32", paren(right, prec::AS)),
            };
            (format!("{}.{method}({count}).unwrap_or(0)", paren(left, prec::POSTFIX)), prec::POSTFIX)
        };
        match unsigned {
            // Back to the signed type.
            Some(_) => (format!("{} as {}", paren(code, prec::AS), kind.rust()), prec::AS),
            None => code,
        }
    }

    /// The Rust place expression for an assignment target.
    pub(super) fn place(&mut self, e: &Expr) -> String {
        self.hoist_target_indexes(e);
        if let Some((self_sym, path)) = self.a.self_fields.get(&e.id) {
            return format!("{}.{}", self.self_place(*self_sym), path.join("."));
        }
        match &e.kind {
            ExprKind::Name(_) => self.var_place(self.a.refs[&e.id]),
            ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::Deref(_) => {
                let saved = std::mem::replace(&mut self.mutable_place, true);
                let mut code = self.selector(e);
                self.mutable_place = saved;
                // A whole record through a pointer, `p^ := ...`: assign
                // through the `RefMut` guard.
                if let ExprKind::Deref(_) = e.kind {
                    code = format!("*{code}");
                }
                code
            }
            _ => unreachable!("sema rejects other assignment targets"),
        }
    }

    /// `record.field` or `array[i][j]`. Pascal's `a[i, j]` becomes Rust's
    /// `a[i][j]`; the runtime's `PArray` does the bounds arithmetic.
    fn selector(&mut self, e: &Expr) -> String {
        match &e.kind {
            // `p^` inside a longer path, like `p^.next`: borrow the record,
            // mutably if we're about to change it. Only the last `^` in a
            // path needs that: in `head^.next^.value := 2`, `head^` is just
            // read to find `next`, so it's `head.get().next.get_mut()`.
            ExprKind::Deref(base) => {
                let method = if self.mutable_place { "get_mut" } else { "get" };
                let saved = std::mem::replace(&mut self.mutable_place, false);
                let base_code = self.selector_base(base);
                self.mutable_place = saved;
                match self.a.type_of(e.id) {
                    // A pointer into a hierarchy with virtual methods points to
                    // its enum; this picks out the object's part.
                    Type::Object(o) if self.in_dispatch(o) => {
                        let suffix = if self.mutable_place { "_mut" } else { "" };
                        format!("{base_code}.{method}().as_{}{suffix}()", super::accessor(o))
                    }
                    _ => format!("{base_code}.{method}()"),
                }
            }
            ExprKind::Field(base, field) => {
                let path = match self.a.type_of(base.id).clone() {
                    Type::Record(r) => r.field(&field.name).expect("sema checked the field").rust_name.clone(),
                    Type::Object(o) => o.field_path(&field.name).expect("sema checked the field").0.join("."),
                    _ => unreachable!("sema checked the field"),
                };
                format!("{}.{path}", self.selector_base(base))
            }
            // `TBytes(Data)[i]`: a byte of an untyped parameter, read or
            // written in place.
            ExprKind::Index(base, indexes)
                if let Some(bytes) = self.view_cast_guard(base)
                    && let Type::Array(a) = self.a.type_of(base.id).clone()
                    && super::super::sema::types::tp_size(&a.elem) == Some(1)
                    && indexes.len() == 1 =>
            {
                let index = self.expr(&indexes[0]).0;
                if a.lo == 0 { format!("{bytes}[{index}]") } else { format!("{bytes}[{index} - {}]", a.lo) }
            }
            ExprKind::Index(base, indexes) => {
                let mut ty = self.a.type_of(base.id).clone();
                let mut code = self.selector_base(base);
                for index in indexes {
                    let index_code = match &ty {
                        // A character literal index must become a byte
                        // literal: `counts['a']` → `counts[b'a']`.
                        Type::Array(a) if a.index == Type::Char => self.coerce(index, &Type::Char).0,
                        _ => self.expr(index).0,
                    };
                    code.push_str(&format!("[{index_code}]"));
                    ty = match &ty {
                        Type::Array(a) => a.elem.clone(),
                        _ => Type::Char,
                    };
                }
                code
            }
            _ => unreachable!(),
        }
    }

    /// The Rust name of an untyped parameter (its `Bytes`).
    pub(super) fn untyped_name(&self, e: &Expr) -> String {
        let sym = self.a.refs[&e.id];
        self.a.sym(sym).rust_name.clone()
    }

    /// If `e` is a typecast of an untyped parameter (`TBytes(Data)`), the
    /// parameter's `Bytes`.
    pub(super) fn view_cast_guard(&self, e: &Expr) -> Option<String> {
        let ExprKind::Call { args, .. } = &e.kind else { return None };
        match args.as_slice() {
            [arg] if *self.a.type_of(arg.id) == Type::Untyped => Some(self.untyped_name(arg)),
            _ => None,
        }
    }

    /// An argument for an untyped `var` parameter: the variable as bytes,
    /// from where the argument starts (`&mut x`, `&mut string_bytes(&mut s,
    /// 30)`, `&mut bytes_from(&mut buf, 9)`).
    pub(super) fn view_arg(&mut self, arg: &Expr) -> String {
        if *self.a.type_of(arg.id) == Type::Untyped {
            return format!("&mut {}", self.untyped_name(arg));
        }
        let (root, offset) = self.view_offset(arg);
        let root_view = match self.a.type_of(root.id).clone() {
            Type::Str { cap } => format!("&mut string_bytes({}, {cap})", self.place_mut(root)),
            _ => self.place_mut(root),
        };
        match offset {
            None => root_view,
            Some(offset) => format!("&mut bytes_from({root_view}, {offset})"),
        }
    }

    /// A variable's bytes to read from (`Move`'s source).
    pub(super) fn view_bytes(&mut self, arg: &Expr) -> String {
        if *self.a.type_of(arg.id) == Type::Untyped {
            return format!("&{}", self.untyped_name(arg));
        }
        let (root, offset) = self.view_offset(arg);
        let all = match self.a.type_of(root.id).clone() {
            Type::Str { cap } => format!("string_view_bytes(&{}, {cap})", paren(self.expr(root), prec::UNARY)),
            _ => format!("{}.to_bytes()", paren(self.expr(root), prec::POSTFIX)),
        };
        match offset {
            None => format!("&{all}"),
            Some(offset) => format!("&{all}[{offset}..]"),
        }
    }

    /// Where `e` starts inside the variable it's part of, in TP's bytes:
    /// the variable, and the offset (`None` for the whole of it). An array
    /// element is `(i - lo) * size` in, a string's `s[i]` is `i` in (after
    /// the length byte), a record field after the fields before it.
    fn view_offset<'e>(&mut self, e: &'e Expr) -> (&'e Expr, Option<String>) {
        use super::super::sema::types::tp_size;
        let mut parts: Vec<String> = Vec::new();
        let mut constant: i64 = 0;
        let mut node = e;
        // Walk from the argument up to the variable, collecting offsets.
        loop {
            match &node.kind {
                ExprKind::Index(base, indexes) => {
                    let mut ty = self.a.type_of(base.id).clone();
                    for index in indexes {
                        let (lo, size, next) = match &ty {
                            Type::Array(a) => (a.lo, tp_size(&a.elem).unwrap_or(1), a.elem.clone()),
                            _ => (0, 1, Type::Char),
                        };
                        let literal = match &index.kind {
                            ExprKind::Int(n) => Some(*n),
                            _ => match self.a.folded.get(&index.id) {
                                Some(crate::sema::ConstVal::Int(n)) => Some(*n),
                                _ => None,
                            },
                        };
                        match literal {
                            Some(n) => constant += (n - lo) * size,
                            None => {
                                let i = paren(self.expr(index), prec::AS);
                                let start =
                                    if lo == 0 { format!("{i} as usize") } else { format!("({i} as usize - {lo})") };
                                parts.push(if size == 1 { start } else { format!("{start} * {size}") });
                            }
                        }
                        ty = next;
                    }
                    node = base;
                }
                ExprKind::Field(base, field) => {
                    if let Type::Record(r) = self.a.type_of(base.id) {
                        let before = r.fields.iter().take_while(|f| !f.name.eq_ignore_ascii_case(&field.name));
                        constant += before.map(|f| tp_size(&f.ty).unwrap_or(0)).sum::<i64>();
                    }
                    node = base;
                }
                _ => break,
            }
        }
        if std::ptr::eq(node, e) {
            return (node, None); // the whole variable
        }
        if constant != 0 || parts.is_empty() {
            parts.push(constant.to_string());
        }
        (node, Some(parts.join(" + ")))
    }

    /// The left side of `.field` or `[index]`. Rust dereferences
    /// automatically there, so a variable we hold by reference can be used
    /// as is (`grid[c][r]`, not `(*grid)[c][r]`).
    pub(super) fn selector_base(&mut self, base: &Expr) -> String {
        if let Some((self_sym, path)) = self.a.self_fields.get(&base.id) {
            return format!("{}.{}", self.self_place(*self_sym), path.join("."));
        }
        match &base.kind {
            // Fields through `shape: &mut dyn TShapeMethods`: `shape.t_shape().x`.
            ExprKind::Name(_) if self.is_dyn_param(base) => {
                let Type::Object(o) = self.a.type_of(base.id).clone() else { unreachable!() };
                format!("{}.{}()", self.a.var_name(self.proc, self.a.refs[&base.id]), super::accessor(&o))
            }
            ExprKind::Name(_) if matches!(self.a.sym(self.a.refs[&base.id]).kind, SymKind::Var(_)) => {
                self.a.var_name(self.proc, self.a.refs[&base.id]).to_string()
            }
            ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::Deref(_) => self.selector(base),
            _ => paren(self.expr(base), prec::POSTFIX),
        }
    }

    /// A mutable reference to a place, for a `var` argument: `&mut x`, or
    /// just `x` if we already hold `x` as a reference (Rust "reborrows" it).
    pub(super) fn place_mut(&mut self, e: &Expr) -> String {
        self.hoist_target_indexes(e);
        // `Report(Output)` or `Report(Lst)` with `procedure Report(var f: Text)`.
        match self.std_text(e) {
            Some(crate::sema::StdFile::Printer) => return "&mut PText::printer()".to_string(),
            Some(_) => return "&mut PText::console()".to_string(),
            None => {}
        }
        match &e.kind {
            ExprKind::Name(_) if !self.a.self_fields.contains_key(&e.id) => {
                let sym = self.a.refs[&e.id];
                let name = self.a.var_name(self.proc, sym).to_string();
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
        if let Some((pid, path)) = self.a.method_calls.get(&callee.id).cloned() {
            return self.method_call(callee, pid, &path, args);
        }
        let sym = self.a.refs[&callee.id];
        match &self.a.sym(sym).kind {
            SymKind::Proc(p) => self.user_call(*p, args),
            SymKind::Builtin(b) => {
                let b = *b;
                let code = self.builtin(b, args, callee.id);
                // Writing to the console never sets `IOResult`, so it needs
                // no `io_unchecked` around it.
                let console_write = matches!(b, Builtin::Write | Builtin::Writeln)
                    && !args.first().is_some_and(|a| matches!(self.a.type_of(a.id), Type::Text | Type::File(_)));
                if b.is_io() && !console_write { self.io_checked(code, callee.span) } else { code }
            }
            SymKind::UnitRoutine(r) => {
                use crate::sema::units::Param;
                let r: &crate::sema::units::UnitRoutine = r;
                let parts: Vec<String> = r
                    .params
                    .iter()
                    .zip(args)
                    .map(|(param, arg)| match param {
                        Param::Int => self.expr(arg).0,
                        Param::Real => self.coerce(arg, &Type::Real).0,
                        Param::Str => self.string_arg(arg),
                        Param::Bool => self.expr(arg).0,
                        Param::FillPattern => self.coerce(arg, &crate::sema::units::fill_pattern_type()).0,
                        Param::VarWord
                        | Param::VarInteger
                        | Param::VarText
                        | Param::VarStr(_)
                        | Param::VarBool
                        | Param::VarSearchRec
                        | Param::VarDateTime
                        | Param::VarLongInt
                        | Param::VarFile
                        | Param::VarFillPattern
                        | Param::VarRecord(_) => self.place_mut(arg),
                        // The points as a `&[PointType]` slice. A method
                        // call dereferences by itself, so a `var` parameter
                        // `*points` is just `points.as_slice()`.
                        Param::Points => {
                            let (code, precedence) = self.expr(arg);
                            match code.strip_prefix('*') {
                                Some(rest) => format!("{rest}.as_slice()"),
                                None => format!("{}.as_slice()", paren((code, precedence), prec::POSTFIX)),
                            }
                        }
                    })
                    .collect();
                let code = format!("{}({})", r.rust, parts.join(", "));
                if matches!(r.rust, "ch_dir" | "mk_dir" | "rm_dir") { self.io_checked(code, callee.span) } else { code }
            }
            // `TRec(Data)`: an untyped parameter's bytes, read as a `TRec`.
            SymKind::TypeName(ty) if *self.a.type_of(args[0].id) == Type::Untyped => {
                let bytes = self.untyped_name(&args[0]);
                format!("{bytes}.read::<{}>()", ty.rust())
            }
            SymKind::TypeName(ty) => {
                let ty = ty.clone();
                let from = self.a.type_of(args[0].id).clone();
                let code = self.coerce(&args[0], &from.concrete());
                cast(code, &from.concrete(), &ty).0
            }
            SymKind::Var(_) | SymKind::Const { .. } | SymKind::RuntimeVar { .. } | SymKind::StdText { .. } => {
                self.expr(callee).0
            }
        }
    }

    /// An I/O call at `span`: as is, or, if it was compiled with `{$I-}`,
    /// wrapped so that an error goes to `IOResult`: `io_unchecked(|| f.reset())`.
    fn io_checked(&mut self, code: String, span: crate::diag::Span) -> String {
        if !self.io_unchecked.iter().any(|r| r.start <= span.start && span.start < r.end) {
            return code;
        }
        if self.first_time("io_unchecked") {
            self.pending.push("// `{$I-}`: an I/O error in `io_unchecked` sets `IOResult` instead of".to_string());
            self.pending.push("// stopping the program.".to_string());
        }
        let body = match code.strip_suffix(';') {
            // A single statement, `f.reset();`, becomes an expression.
            Some(stmt) if !stmt.contains(';') => stmt.to_string(),
            Some(_) => format!("{{ {code} }}"),
            None => code,
        };
        format!("io_unchecked(|| {body})")
    }

    pub(super) fn user_call(&mut self, p: usize, args: &[Expr]) -> String {
        let (args, records) = self.call_args(p, args, None);
        wrap_with_mut(format!("{}({args})", self.a.procs[p].rust_name), records)
    }

    /// `receiver.method(args)`. The receiver is the object before the dot
    /// (`queue.add_q(item)`), or `self` for a call from a sibling method;
    /// `path` reaches an inherited method's object (`board.base.show()`).
    fn method_call(&mut self, callee: &Expr, pid: usize, path: &[String], args: &[Expr]) -> String {
        // `inherited Draw` or `TShape.Init(...)`: that version, on `Self`.
        let static_call = self.a.static_calls.contains(&callee.id);
        // A method of a trait (see `object_traits`) is called on the whole
        // object, which picks the version; no path to an ancestor.
        let dispatched = self.a.dispatched.contains(&pid);
        let mut receiver = match &callee.kind {
            ExprKind::Field(receiver, _) if !static_call => match &receiver.kind {
                ExprKind::Name(_) if dispatched && self.is_dyn_param(receiver) => {
                    self.a.var_name(self.proc, self.a.refs[&receiver.id]).to_string()
                }
                // `p^.Draw` through a pointer into the hierarchy: the enum
                // finds the version.
                ExprKind::Deref(pointer) if dispatched => format!("{}.get_mut()", self.selector_base(pointer)),
                // The method takes `&mut self`, so borrow a record behind a
                // pointer mutably.
                _ => {
                    let saved = std::mem::replace(&mut self.mutable_place, true);
                    let code = self.selector_base(receiver);
                    self.mutable_place = saved;
                    code
                }
            },
            _ => {
                let self_sym = self.a.procs[self.proc_with_self()].self_param.as_ref().unwrap().1;
                if dispatched { self.a.var_name(self.proc, self_sym).to_string() } else { self.self_place(self_sym) }
            }
        };
        if !dispatched {
            for step in path {
                receiver.push('.');
                receiver.push_str(step);
            }
        }
        let name = if static_call { self.body_name(pid) } else { self.a.procs[pid].rust_name.clone() };
        // The receiver is borrowed (`&mut self`) for the whole call, so an
        // argument that uses the same object, `obj.Show(obj.Name)`, must be
        // computed first. With an implicit `Self`, any call in an argument
        // might be another method of the same object.
        let borrowed = match &callee.kind {
            ExprKind::Field(receiver, _) if !static_call => Receiver::Named(root_key(receiver)),
            _ => Receiver::ImplicitSelf,
        };
        let (args, records) = self.call_args(pid, args, Some(borrowed));
        wrap_with_mut(format!("{receiver}.{name}({args})"), records)
    }

    /// How code reaches `Self`'s fields: `self`, or in a trait's default
    /// method `self.t_shape()` (see `object_traits`).
    fn self_place(&self, self_sym: SymId) -> String {
        match &self.self_expr {
            Some(expr) => expr.clone(),
            None => self.a.var_name(self.proc, self_sym).to_string(),
        }
    }

    /// Is `e` a `var` parameter that takes any object of a hierarchy with
    /// virtual methods (`shape: &mut dyn TShapeMethods`)?
    pub(super) fn is_dyn_param(&self, e: &Expr) -> bool {
        let ExprKind::Name(_) = &e.kind else { return false };
        let Some(&sym) = self.a.refs.get(&e.id) else { return false };
        if self.a.sym(sym).rust_name == "self" {
            return false;
        }
        match &self.a.sym(sym).kind {
            SymKind::Var(v) => {
                v.role == crate::sema::VarRole::Param(ParamMode::Var)
                    && matches!(&v.ty, Type::Object(o) if self.in_dispatch(o))
            }
            _ => false,
        }
    }

    /// The method we're in (or nested inside), whose `Self` a bare method
    /// call uses.
    fn proc_with_self(&self) -> usize {
        let mut p = self.proc;
        while self.a.procs[p].self_param.is_none() {
            p = self.a.procs[p].parent.expect("a bare method call happens inside a method");
        }
        p
    }

    /// The argument list for a call: captured outer variables first, then
    /// the Pascal arguments. Also returns the heap records that two or more
    /// `var` arguments are fields of, as `(pointer, name)` pairs: the call
    /// must be wrapped in `pointer.with_mut(|name| ...)` (see
    /// [`wrap_with_mut`]).
    fn call_args(&mut self, p: usize, args: &[Expr], receiver: Option<Receiver>) -> (String, Vec<(String, String)>) {
        let info = &self.a.procs[p];
        let mut parts = Vec::new();
        // Captured variables first, in the callee's order.
        for cap in &info.captures {
            let mut name = self.a.var_name(self.proc, cap.sym).to_string();
            if name == "self"
                && let Some(expr) = &self.self_expr
            {
                name = expr.clone();
            }
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
        // Variables this call borrows mutably (`var` arguments).
        let borrowed: Vec<String> = params
            .iter()
            .zip(args)
            .filter(|(p, _)| self.a.var(**p).role == crate::sema::VarRole::Param(ParamMode::Var))
            .map(|(_, arg)| root_key(arg))
            .collect();
        // Heap records shared by more than one `var` argument (`q^.x, q^.y`).
        let is_var = |p: &usize| self.a.var(*p).role == crate::sema::VarRole::Param(ParamMode::Var);
        let heap: Vec<Option<String>> = params
            .iter()
            .zip(args)
            .map(|(p, arg)| if is_var(p) { crate::sema::heap_field(arg).map(|(pointer, ..)| pointer) } else { None })
            .collect();
        let shared = |key: &Option<String>| key.is_some() && heap.iter().filter(|k| *k == key).count() > 1;
        let mut records: Vec<(String, String, String)> = Vec::new(); // (key, pointer, name)
        // Value arguments not yet in a `let`: (position in `parts`,
        // parameter, whether it calls something). If a later argument has to
        // go in a `let` and one of these calls something, they all go
        // first, in order, so the arguments still run left to right, as TP
        // pushed them: in `Show(x, Bump, p^.v)`, `x` is read before `Bump`.
        let mut values_before: Vec<(usize, usize, bool)> = Vec::new();
        // Whether an earlier `var` argument is borrowed through a pointer
        // (`Add(p^.x, F)`): it's borrowed for the whole call, so a later
        // argument that calls something (which may change the record) is
        // worked out into a `let` first.
        let mut pointer_var_before = false;
        for ((param, arg), key) in params.iter().zip(args).zip(&heap) {
            let var = self.a.var(*param);
            let ty = var.ty.clone();
            let is_var_arg = var.role == crate::sema::VarRole::Param(ParamMode::Var);
            // An object passed where an ancestor is expected: its ancestor
            // part, `&mut circle.base`. (With virtual methods, the parameter
            // takes the whole object, `&mut dyn TShapeMethods`.)
            let depth = match (&ty, self.a.type_of(arg.id)) {
                (Type::Object(param), Type::Object(given)) if !self.in_dispatch(param) => {
                    given.lineage().iter().position(|(o, _)| o == param).unwrap_or(0)
                }
                _ => 0,
            };
            // `Paint(p^)` with `var s: TShape`: the whole record behind the
            // pointer (the hierarchy's enum, which implements every
            // object's trait), so the record's real type picks the methods.
            let dyn_param = matches!(&ty, Type::Object(o) if self.in_dispatch(o));
            if ty == Type::Untyped {
                parts.push(self.view_arg(arg));
            } else if var.role == crate::sema::VarRole::Param(ParamMode::Var)
                && dyn_param
                && let ExprKind::Deref(pointer) = &arg.kind
            {
                parts.push(format!("&mut *{}.get_mut()", self.selector_base(pointer)));
            } else if var.role == crate::sema::VarRole::Param(ParamMode::Var) && depth > 0 {
                parts.push(format!("&mut {}{}", self.selector_base(arg), ".base".repeat(depth)));
            } else if var.role == crate::sema::VarRole::Param(ParamMode::Var) && shared(key) {
                // `&mut q.get_mut().x` becomes `&mut point.x`, where `point`
                // is the record borrowed once by `with_mut`.
                let place = self.place(arg);
                let (pointer, fields) = place.rsplit_once(".get_mut()").expect("a heap field");
                let key = key.clone().unwrap();
                let name = match records.iter().find(|(k, ..)| *k == key) {
                    Some((_, _, name)) => name.clone(),
                    None => {
                        let mut base = self.record_name(arg);
                        if !records.is_empty() {
                            base = format!("{base}_{}", records.len() + 1);
                        }
                        let name = self.temp_name(&base);
                        records.push((key, pointer.to_string(), name.clone()));
                        name
                    }
                };
                parts.push(format!("&mut {name}{fields}"));
            } else if var.role == crate::sema::VarRole::Param(ParamMode::Var) {
                parts.push(self.place_mut(arg));
            } else {
                let code = self.coerce(arg, &ty).0;
                // `f(&mut x, x.size)` borrows `x` mutably and then reads it,
                // which Rust rejects; Pascal evaluates the argument first
                // anyway. Compute it into a temporary before the call.
                //
                // The same goes for an argument that reads through a pointer
                // (`f(p^)` is `f(p.get().clone())`): the borrow guard that
                // `get()` returns would live until the end of the statement,
                // keeping the record borrowed while `f` runs, so `f` couldn't
                // modify or dispose it.
                let uses_receiver = match &receiver {
                    Some(Receiver::Named(root)) => !root.is_empty() && mentions(arg, root),
                    Some(Receiver::ImplicitSelf) => self.calls(arg),
                    None => false,
                };
                // (A constant, like `SizeOf(x)`, reads nothing at run time.)
                let constant = self.a.folded.contains_key(&arg.id);
                if !constant && borrowed.iter().any(|root| !root.is_empty() && mentions(arg, root))
                    || derefs(arg)
                    || uses_receiver
                    || (pointer_var_before && self.has_call(arg))
                {
                    // `ShowV(Bump, p^.value)`: Bump runs before `p^.value`
                    // is read, as in Pascal.
                    let earlier = std::mem::take(&mut values_before);
                    if earlier.iter().any(|&(_, _, calls)| calls) {
                        for (position, earlier_param, _) in earlier {
                            let temp = self.temp_name(&self.a.sym(earlier_param).rust_name.clone());
                            let earlier_code = std::mem::replace(&mut parts[position], temp.clone());
                            self.pending.push(format!("let {temp} = {earlier_code};"));
                        }
                    }
                    let temp = self.temp_name(&self.a.sym(*param).rust_name.clone());
                    self.pending.push(format!("let {temp} = {code};"));
                    parts.push(temp);
                } else {
                    // A literal can't change, so it needn't move.
                    if !matches!(arg.kind, ExprKind::Int(_) | ExprKind::Real(_) | ExprKind::Str(_)) {
                        values_before.push((parts.len(), *param, self.has_call(arg)));
                    }
                    parts.push(code);
                }
            }
            if is_var_arg && super::contains_deref(arg) {
                pointer_var_before = true;
            }
        }
        (parts.join(", "), records.into_iter().map(|(_, pointer, name)| (pointer, name)).collect())
    }

    /// A name for the heap record that `arg` (`q^.x`) is a field of, after
    /// its type: `point` for a `Point`.
    fn record_name(&self, arg: &Expr) -> String {
        let mut e = arg;
        while let ExprKind::Field(base, _) = &e.kind {
            e = base;
        }
        match self.a.type_of(e.id) {
            Type::Record(r) => crate::sema::names::snake_case(&r.rust_name),
            Type::Object(o) => crate::sema::names::snake_case(&o.rust_name),
            _ => "rec".to_string(),
        }
    }

    /// Does `e` contain a call (which might be a method call)? A method
    /// without parameters can be called without parentheses, as a bare name.
    fn calls(&self, e: &Expr) -> bool {
        if self.a.method_calls.contains_key(&e.id) {
            return true;
        }
        match &e.kind {
            ExprKind::Call { .. } => true,
            ExprKind::Field(base, _) | ExprKind::Deref(base) => self.calls(base),
            ExprKind::Index(base, indexes) => self.calls(base) || indexes.iter().any(|i| self.calls(i)),
            ExprKind::Unary { operand, .. } => self.calls(operand),
            ExprKind::Binary { lhs, rhs, .. } => self.calls(lhs) || self.calls(rhs),
            _ => false,
        }
    }

    /// A name for a temporary, based on `base` (a parameter's or field's
    /// name): `base` itself, unless that would shadow one of the caller's
    /// variables (then `base_arg`) or another temporary already made for the
    /// same statement (then `base_2`, `base_3`, ...).
    pub(super) fn temp_name(&self, base: &str) -> String {
        let info = &self.a.procs[self.proc];
        let visible = info.locals.iter().chain(&info.params).chain(info.captures.iter().map(|c| &c.sym));
        let base = if visible.into_iter().any(|sym| self.a.var_name(self.proc, *sym) == base) {
            format!("{base}_arg")
        } else {
            base.to_string()
        };
        let taken = |name: &str| self.pending.iter().any(|line| line.starts_with(&format!("let {name} = ")));
        let mut name = base.clone();
        let mut n = 1;
        while taken(&name) {
            n += 1;
            name = format!("{base}_{n}");
        }
        name
    }

    /// The Rust literal for a constant of type `ty`.
    pub(super) fn const_value(&self, value: &ConstVal, ty: &Type) -> String {
        match value {
            ConstVal::Int(n) => match ty {
                Type::Enum(e) => format!("{}::{}", e.rust_name, e.variants[*n as usize]),
                Type::Char => char_literal(*n as u8),
                _ => n.to_string(),
            },
            ConstVal::Real(x) => real_literal(*x),
            ConstVal::Bool(b) => b.to_string(),
            ConstVal::Str(bytes) if *ty == Type::Char && bytes.len() == 1 => char_literal(bytes[0]),
            ConstVal::Str(bytes) => str_literal(bytes),
            // `PSet::from_ranges` is a `const fn`, so it can initialize a
            // `const`; it takes the elements' ordinals as `u8`s.
            ConstVal::Set(ranges) => {
                let elem_ty = match ty {
                    Type::Set(elem) => elem.concrete(),
                    _ => Type::Int(IntKind::Byte),
                };
                let ord = |n: u8| match &elem_ty {
                    Type::Char => char_literal(n),
                    Type::Enum(e) => format!("{}::{} as u8", e.rust_name, e.variants[n as usize]),
                    _ => n.to_string(),
                };
                if ranges.iter().all(|(lo, hi)| lo == hi) {
                    let elements: Vec<String> = ranges.iter().map(|&(n, _)| ord(n)).collect();
                    return format!("PSet::of(&[{}])", elements.join(", "));
                }
                let ranges: Vec<String> =
                    ranges.iter().map(|&(lo, hi)| format!("({}, {})", ord(lo), ord(hi))).collect();
                format!("PSet::from_ranges(&[{}])", ranges.join(", "))
            }
        }
    }
}

/// Wrap a call in `pointer.with_mut(|name| call)` for each heap record that
/// several of its `var` arguments are fields of.
fn wrap_with_mut(call: String, records: Vec<(String, String)>) -> String {
    records.into_iter().rev().fold(call, |call, (pointer, name)| format!("{pointer}.with_mut(|{name}| {call})"))
}

/// The lowercase name of the variable a place starts from (`a` for
/// `a[i].x`), or empty.
fn root_key(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Name(id) => id.key(),
        ExprKind::Field(base, _) | ExprKind::Index(base, _) | ExprKind::Deref(base) => root_key(base),
        _ => String::new(),
    }
}

/// Does the expression mention the name `key` anywhere?
fn mentions(e: &Expr, key: &str) -> bool {
    match &e.kind {
        ExprKind::Name(id) => id.key() == key,
        ExprKind::Field(base, _) | ExprKind::Deref(base) => mentions(base, key),
        ExprKind::Index(base, indexes) => mentions(base, key) || indexes.iter().any(|i| mentions(i, key)),
        ExprKind::Call { callee, args } => mentions(callee, key) || args.iter().any(|a| mentions(a, key)),
        ExprKind::Unary { operand, .. } => mentions(operand, key),
        ExprKind::Binary { lhs, rhs, .. } => mentions(lhs, key) || mentions(rhs, key),
        ExprKind::Format { value, .. } => mentions(value, key),
        _ => false,
    }
}

/// The object a method is called on, for [`Codegen::call_args`].
enum Receiver {
    /// `obj.Method(...)`: the variable the receiver starts from.
    Named(String),
    /// `Method(...)` inside another method of the same object.
    ImplicitSelf,
}

/// Does `e` read through a pointer (`p^`)?
fn derefs(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Deref(_) => true,
        ExprKind::Field(base, _) => derefs(base),
        ExprKind::Index(base, indexes) => derefs(base) || indexes.iter().any(derefs),
        ExprKind::Call { args, .. } => args.iter().any(derefs),
        ExprKind::Unary { operand, .. } => derefs(operand),
        ExprKind::Binary { lhs, rhs, .. } => derefs(lhs) || derefs(rhs),
        _ => false,
    }
}

/// Is `e` a variable or a part of one (as opposed to a computed value)?
pub(super) fn is_place(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Name(_) | ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::Deref(_))
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
        (Type::Int(_) | Type::IntLit(_) | Type::Char | Type::Bool | Type::Enum(_), Type::Int(_) | Type::Char) => {
            cast(code, &from.concrete(), to)
        }
        (Type::Int(_) | Type::IntLit(_), Type::Real) => (format!("f64::from({})", code.0), prec::POSTFIX),
        (Type::StrLit(_) | Type::Char, Type::Str { .. }) => (format!("pstr({})", code.0), prec::POSTFIX),
        _ => code,
    }
}

/// An explicit conversion between ordinal types. Widening conversions that
/// can't lose information use `From` (`i32::from(x)`), which Rust
/// programmers prefer because it can never silently truncate; narrowing
/// ones use `as`, which truncates exactly like Turbo Pascal does.
pub(super) fn cast(code: Code, from: &Type, to: &Type) -> Code {
    // `Char` and `Byte` are both `u8` in Rust: nothing to convert.
    if from == to
        || (matches!(from, Type::Char | Type::Int(_))
            && matches!(to, Type::Char | Type::Int(_))
            && from.rust() == to.rust())
    {
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
    // `Boolean(n)` is true for any nonzero value; Rust has no `as bool`.
    if *to == Type::Bool {
        return (format!("{} != 0", paren(code, prec::CMP + 1)), prec::CMP);
    }
    if let Type::Enum(_) = to {
        return (format!("{rust}::from_ord({})", code.0), prec::POSTFIX);
    }
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
    let prefix = if bytes.iter().all(|b| b.is_ascii()) { "" } else { "b" };
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

/// Does `e` read through a pointer itself, outside any call? (A call's
/// arguments that read through a pointer are put into `let`s by the call.)
fn reads_pointer(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Deref(_) => true,
        ExprKind::Field(base, _) => reads_pointer(base),
        ExprKind::Index(base, indexes) => reads_pointer(base) || indexes.iter().any(reads_pointer),
        ExprKind::Unary { operand, .. } => reads_pointer(operand),
        ExprKind::Binary { lhs, rhs, .. } => reads_pointer(lhs) || reads_pointer(rhs),
        _ => false,
    }
}
