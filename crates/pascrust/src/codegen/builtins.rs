//! Calls to Turbo Pascal's built-in procedures and functions.

use super::Codegen;
use super::expr::{convert, paren, prec};
use crate::ast::*;
use crate::sema::{Builtin, IntKind, Type};

impl<'a> Codegen<'a> {
    /// Translate a builtin call. Statement-like builtins (`Inc`, `Exit`, ...)
    /// return code ending in `;` so the caller doesn't add another.
    pub(super) fn builtin(&mut self, b: Builtin, args: &[Expr]) -> String {
        use Builtin::*;
        let ty = |g: &Self, e: &Expr| g.a.type_of(e.id).clone();
        match b {
            Write | Writeln => {
                let parts: Vec<String> = args.iter().map(|a| self.write_arg(a)).collect();
                let mac = if b == Write { "pwrite!" } else { "pwriteln!" };
                format!("{mac}({})", parts.join(", "))
            }
            Read | Readln => {
                let parts: Vec<String> = args.iter().map(|a| self.place_mut(a)).collect();
                let mac = if b == Read { "pread!" } else { "preadln!" };
                format!("{mac}({})", parts.join(", "))
            }
            Halt => match args.first() {
                None => "halt(0)".to_string(),
                Some(code) => format!("halt({})", self.coerce(code, &Type::Int(IntKind::LongInt)).0),
            },
            Exit => {
                if self.a.procs[self.proc].result.is_some() {
                    "return result;".to_string()
                } else {
                    "return;".to_string()
                }
            }
            Break => "break;".to_string(),
            Continue => "continue;".to_string(),
            Ord => {
                let result = self.call_type(args);
                let from = ty(self, &args[0]).concrete();
                let code = self.coerce(&args[0], &from);
                super::expr::cast(code, &from, &result).0
            }
            Chr => {
                if let ExprKind::Int(n) = args[0].kind
                    && (0..=255).contains(&n)
                {
                    return super::expr::char_literal(n as u8);
                }
                let from = ty(self, &args[0]).concrete();
                let code = self.coerce(&args[0], &from);
                super::expr::cast(code, &from, &Type::Char).0
            }
            Odd => {
                let t = ty(self, &args[0]).concrete();
                format!("{} % 2 != 0", paren(self.coerce(&args[0], &t), prec::MUL))
            }
            Succ | Pred => {
                let t = ty(self, &args[0]).concrete();
                if let Type::Enum(_) = t {
                    let method = if b == Succ { "succ" } else { "pred" };
                    return format!("{}.{method}()", self.postfix(&args[0], &t));
                }
                let op = if b == Succ { "+" } else { "-" };
                format!("{} {op} 1", paren(self.coerce(&args[0], &t), prec::ADD))
            }
            Inc | Dec => {
                let target_ty = ty(self, &args[0]);
                let place = self.place(&args[0]);
                let amount = match args.get(1) {
                    Some(n) => self.coerce(n, &target_ty).0,
                    None => "1".to_string(),
                };
                let op = if b == Inc { "+=" } else { "-=" };
                format!("{place} {op} {amount};")
            }
            Hi => {
                let t = ty(self, &args[0]).concrete();
                format!("({} >> 8) as u8", paren(self.coerce(&args[0], &t), prec::SHIFT + 1))
            }
            Lo => {
                let t = ty(self, &args[0]).concrete();
                format!("{} as u8", paren(self.coerce(&args[0], &t), prec::AS))
            }
            Abs => {
                let t = ty(self, &args[0]).concrete();
                format!("{}.abs()", self.postfix(&args[0], &t))
            }
            Sqr => {
                let t = ty(self, &args[0]).concrete();
                let method = if t == Type::Real { "powi" } else { "pow" };
                format!("{}.{method}(2)", self.postfix(&args[0], &t))
            }
            Sqrt | Sin | Cos | ArcTan | Ln | Exp | Int | Frac => {
                let method = match b {
                    Sqrt => "sqrt",
                    Sin => "sin",
                    Cos => "cos",
                    ArcTan => "atan",
                    Ln => "ln",
                    Exp => "exp",
                    Int => "trunc",
                    _ => "fract",
                };
                format!("{}.{method}()", self.postfix(&args[0], &Type::Real))
            }
            Trunc | Round => {
                let method = if b == Trunc { "trunc" } else { "round" };
                format!("{}.{method}() as i32", self.postfix(&args[0], &Type::Real))
            }
            Pi => "std::f64::consts::PI".to_string(),
            Random => match args.first() {
                Some(n) => format!("random({})", self.expr(n).0),
                None => "random_real()".to_string(),
            },
            Randomize => "randomize()".to_string(),
            UpCase => format!("{}.to_ascii_uppercase()", self.postfix(&args[0], &Type::Char)),
            Length => match ty(self, &args[0]) {
                Type::StrLit(n) => n.to_string(),
                Type::Char => "1".to_string(),
                _ => format!("{}.length()", self.postfix(&args[0], &Type::Str { cap: 255 })),
            },
            Copy => {
                let s = self.postfix(&args[0], &Type::Str { cap: 255 });
                let (i, n) = (self.expr(&args[1]).0, self.expr(&args[2]).0);
                format!("{s}.copy({i}, {n})")
            }
            Pos => {
                let (sub, s) = (self.string_arg(&args[0]), self.string_arg(&args[1]));
                format!("pos({sub}, {s})")
            }
            Concat => {
                let mut parts = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    parts.push(if i == 0 {
                        self.coerce(arg, &Type::Str { cap: 255 }).0
                    } else {
                        paren(self.expr(arg), prec::ADD + 1)
                    });
                }
                parts.join(" + ")
            }
            Delete => {
                let s = self.place_receiver(&args[0]);
                let (i, n) = (self.expr(&args[1]).0, self.expr(&args[2]).0);
                format!("{s}.delete({i}, {n});")
            }
            Insert => {
                let src = self.string_arg(&args[0]);
                let s = self.place_receiver(&args[1]);
                let i = self.expr(&args[2]).0;
                format!("{s}.insert({src}, {i});")
            }
            Str => {
                let value = self.write_arg(&args[0]);
                let place = self.place(&args[1]);
                format!("{place} = pas_str({value});")
            }
            Val => {
                let s = self.string_arg(&args[0]);
                let (x, code) = (self.place_mut(&args[1]), self.place_mut(&args[2]));
                format!("val({s}, {x}, {code});")
            }
        }
    }

    /// The result type sema computed for this builtin call. The call
    /// expression's id isn't passed in, so recompute it the same way sema did.
    fn call_type(&self, args: &[Expr]) -> Type {
        match self.a.type_of(args[0].id).concrete() {
            Type::Int(IntKind::LongInt) => Type::Int(IntKind::LongInt),
            Type::Int(IntKind::Word) => Type::Int(IntKind::Word),
            _ => Type::Int(IntKind::Integer),
        }
    }

    /// One argument of `Write`/`Writeln`/`Str`, with its `:width:decimals`.
    fn write_arg(&mut self, arg: &Expr) -> String {
        let (value, width, decimals) = match &arg.kind {
            ExprKind::Format { value, width, decimals } => (&**value, Some(&**width), decimals.as_deref()),
            _ => (arg, None, None),
        };
        let ty = self.a.type_of(value.id).clone();
        let code = match ty {
            // `Byte` is a `u8` in Rust, which the runtime prints as a
            // character (that's what `Char` is). Widen it so it prints as a
            // number.
            Type::Int(IntKind::Byte) => convert(self.expr(value), &ty, &Type::Int(IntKind::Integer)).0,
            _ => self.expr(value).0,
        };
        match (width, decimals) {
            (None, _) => code,
            (Some(w), None) => format!("w({code}, {})", self.expr(w).0),
            (Some(w), Some(d)) => format!("wd({code}, {}, {})", self.expr(w).0, self.expr(d).0),
        }
    }

    /// A string-ish argument for a runtime function that accepts anything
    /// convertible to a `PString` (`&str`, `u8`, `PString`).
    fn string_arg(&mut self, e: &Expr) -> String {
        if let ExprKind::Str(bytes) = &e.kind {
            return super::expr::str_literal(bytes);
        }
        self.expr(e).0
    }

    /// A variable used as the receiver of a mutating method (`s.delete(..)`).
    /// Method calls auto-dereference, so references don't need a `*`.
    fn place_receiver(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Name(_) => self.a.var_name(self.proc, self.a.refs[&e.id]).to_string(),
            _ => self.place(e),
        }
    }
}
