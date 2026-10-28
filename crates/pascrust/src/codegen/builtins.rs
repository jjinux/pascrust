//! Calls to Turbo Pascal's built-in procedures and functions.

use super::expr::{convert, paren, prec};
use super::{Codegen, LoopLabels};
use crate::ast::*;
use crate::sema::{Builtin, IntKind, SymKind, Type};

impl<'a> Codegen<'a> {
    /// Translate a builtin call. Statement-like builtins (`Inc`, `Exit`, ...)
    /// return code ending in `;` so the caller doesn't add another.
    /// A variable for `Read` to fill in. A `string[n]` takes at most `n`
    /// characters, which the runtime's `capped(n)` arranges.
    fn read_target(&mut self, e: &Expr) -> String {
        match self.a.type_of(e.id) {
            Type::Str { cap } if *cap < 255 => {
                let cap = *cap;
                let place = match &e.kind {
                    ExprKind::Name(_) if !self.a.self_fields.contains_key(&e.id) => {
                        self.a.var_name(self.proc, self.a.refs[&e.id]).to_string()
                    }
                    _ => {
                        // `p^` is the place `*p.get_mut()`, which needs
                        // parentheses before `.capped`.
                        let place = self.place(e);
                        let p = if place.starts_with('*') { prec::UNARY } else { prec::POSTFIX };
                        paren((place, p), prec::POSTFIX)
                    }
                };
                format!("&mut {place}.capped({cap})")
            }
            _ => self.place_mut(e),
        }
    }

    /// `call_id` is the callee's id, which sema's notes about the call use.
    pub(super) fn builtin(&mut self, b: Builtin, args: &[Expr], call_id: NodeId) -> String {
        use Builtin::*;
        let ty = |g: &Self, e: &Expr| g.a.type_of(e.id).clone();
        match b {
            Write | Writeln => match args.first().map(|a| ty(self, a)) {
                // `Write(f, x, y)` on a typed file writes whole records.
                Some(Type::File(_)) => {
                    let file = self.place_receiver(&args[0]);
                    let writes: Vec<String> =
                        args[1..].iter().map(|a| format!("{file}.write(&{});", self.expr(a).0)).collect();
                    writes.join(" ")
                }
                Some(Type::Text) => {
                    let mut parts = vec![self.place_receiver(&args[0])];
                    parts.extend(args[1..].iter().map(|a| self.write_arg(a)));
                    let mac = if b == Write { "fwrite!" } else { "fwriteln!" };
                    format!("{mac}({})", parts.join(", "))
                }
                _ => {
                    let parts: Vec<String> = args.iter().map(|a| self.write_arg(a)).collect();
                    let mac = if b == Write { "pwrite!" } else { "pwriteln!" };
                    format!("{mac}({})", parts.join(", "))
                }
            },
            Read | Readln => match args.first().map(|a| ty(self, a)) {
                Some(Type::File(_)) => {
                    let file = self.place_receiver(&args[0]);
                    let reads: Vec<String> =
                        args[1..].iter().map(|a| format!("{file}.read({});", self.place_mut(a))).collect();
                    reads.join(" ")
                }
                Some(Type::Text) => {
                    let mut parts = vec![self.place_receiver(&args[0])];
                    parts.extend(args[1..].iter().map(|a| self.read_target(a)));
                    let mac = if b == Read { "fread!" } else { "freadln!" };
                    format!("{mac}({})", parts.join(", "))
                }
                _ => {
                    let parts: Vec<String> = args.iter().map(|a| self.read_target(a)).collect();
                    let mac = if b == Read { "pread!" } else { "preadln!" };
                    format!("{mac}({})", parts.join(", "))
                }
            },
            Assign | Rename => {
                let file = self.place_receiver(&args[0]);
                let method = if b == Assign { "assign" } else { "rename" };
                format!("{file}.{method}({});", self.string_arg(&args[1]))
            }
            Reset | Rewrite | Append | Close | Erase | Flush | Truncate => {
                let method = format!("{b:?}").to_ascii_lowercase();
                format!("{}.{method}();", self.place_receiver(&args[0]))
            }
            Seek => format!("{}.seek({});", self.place_receiver(&args[0]), self.expr(&args[1]).0),
            FilePos => format!("{}.file_pos()", self.place_receiver(&args[0])),
            FileSize => format!("{}.file_size()", self.place_receiver(&args[0])),
            Eof | Eoln | SeekEof | SeekEoln => {
                let method = match b {
                    Eof => "eof",
                    Eoln => "eoln",
                    SeekEof => "seek_eof",
                    _ => "seek_eoln",
                };
                match args.first() {
                    Some(file) => format!("{}.{method}()", self.place_receiver(file)),
                    None => format!("{method}()"),
                }
            }
            IOResult => "io_result()".to_string(),
            TypeOf => self.type_of(&args[0]),
            Include | Exclude => {
                let Type::Set(elem) = self.a.type_of(args[0].id).clone() else { unreachable!("sema checked the set") };
                let method = if b == Include { "include" } else { "exclude" };
                format!("{}.{method}({});", self.place_receiver(&args[0]), self.coerce(&args[1], &elem).0)
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
            Break => match self.loops.last() {
                Some(Some(LoopLabels { loop_label: Some(label), .. })) => format!("break {label};"),
                _ => "break;".to_string(),
            },
            Continue => match self.loops.last() {
                Some(Some(labels)) => format!("break {};", labels.body_label),
                _ => "continue;".to_string(),
            },
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
            Swap => {
                let t = ty(self, &args[0]).concrete();
                format!("{}.swap_bytes()", self.postfix(&args[0], &t))
            }
            Assigned => format!("!{}.is_nil()", self.selector_base(&args[0])),
            RunError => match args.first() {
                None => "run_error(0)".to_string(),
                Some(code) => format!("run_error({})", self.expr(code).0),
            },
            SizeOf | High | Low => unreachable!("sema folds these into constants"),
            FillChar => self.fill_char(call_id, args),
            Move => {
                // Sema allows only a whole variable to one of the same type.
                let ty = self.a.type_of(args[1].id).clone();
                format!("{} = {};", self.place(&args[1]), self.coerce(&args[0], &ty).0)
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
            New => self.new_call(call_id, args),
            Dispose => {
                let dispose = format!("{}.dispose();", self.place_receiver(&args[0]));
                match self.a.object_calls.get(&call_id) {
                    // `Dispose(p, Done)`: the destructor first.
                    Some((done, _)) => format!("{}; {dispose}", self.object_call(done)),
                    None => dispose,
                }
            }
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
    pub(super) fn string_arg(&mut self, e: &Expr) -> String {
        if let ExprKind::Str(bytes) = &e.kind {
            return super::expr::str_literal(bytes);
        }
        self.expr(e).0
    }

    /// A variable used as the receiver of a mutating method (`s.delete(..)`).
    /// Method calls auto-dereference, so references don't need a `*`.
    pub(super) fn place_receiver(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Name(_) if !self.a.self_fields.contains_key(&e.id) => {
                self.a.var_name(self.proc, self.a.refs[&e.id]).to_string()
            }
            _ => self.place(e),
        }
    }
}

impl Codegen<'_> {
    /// `New(p)`, `New(p, Init(1))`, or `New` as a function,
    /// `New(PCircle, Init(2))`, which makes the record in a new variable.
    fn new_call(&mut self, call_id: NodeId, args: &[Expr]) -> String {
        match self.a.object_calls.get(&call_id) {
            Some((init, Some(temp))) => {
                let name = self.a.var_name(self.proc, *temp).to_string();
                let alloc = self.alloc(&self.a.var(*temp).ty.clone());
                self.pending.push(format!("{name} = {alloc};"));
                if !matches!(init.kind, ExprKind::Nil) {
                    let init = self.object_call(init);
                    self.pending.push(format!("{init};"));
                }
                format!("{name}.clone()")
            }
            init => {
                let place = self.place(&args[0]);
                let alloc = self.alloc(&self.a.type_of(args[0].id).clone());
                match init {
                    Some((init, None)) => format!("{place} = {alloc}; {};", self.object_call(init)),
                    _ => format!("{place} = {alloc};"),
                }
            }
        }
    }

    /// A constructor or destructor call that sema made up for `New` or
    /// `Dispose` (a method call on `p^`).
    fn object_call(&mut self, call: &Expr) -> String {
        let ExprKind::Call { callee, args } = &call.kind else { unreachable!("sema makes a call") };
        self.call(callee, args)
    }

    /// A new record for a pointer of type `ty`. In a hierarchy with virtual
    /// methods it's the enum's variant for the pointer's object.
    fn alloc(&self, ty: &Type) -> String {
        match ty {
            Type::Pointer(p) => match p.target() {
                Type::Object(o) if self.in_dispatch(&o) => {
                    let root = o.lineage().last().unwrap().0.clone();
                    format!(
                        "Ptr::new({}::{}({}::default()))",
                        super::enum_name(&root),
                        super::variant_name(&o),
                        o.rust_name
                    )
                }
                _ => "Ptr::alloc()".to_string(),
            },
            _ => "Ptr::alloc()".to_string(),
        }
    }
}

impl Codegen<'_> {
    /// `TypeOf(x)`: the name of the object type `x` really is. It's only
    /// known at run time for an object reached through a pointer, a `var`
    /// parameter, or `Self` in a trait's default method; anything else is
    /// exactly its declared type.
    fn type_of(&mut self, arg: &Expr) -> String {
        let Type::Object(object) = self.a.type_of(arg.id).clone() else { return "\"\"".to_string() };
        let is_type_name =
            self.a.refs.get(&arg.id).is_some_and(|&sym| matches!(self.a.sym(sym).kind, SymKind::TypeName(_)));
        if is_type_name || !self.in_dispatch(&object) {
            return format!("\"{}\"", object.rust_name);
        }
        match &arg.kind {
            ExprKind::Deref(pointer) => format!("{}.get().type_name()", self.selector_base(pointer)),
            ExprKind::Name(_) if self.is_dyn_param(arg) => {
                format!("{}.type_name()", self.a.var_name(self.proc, self.a.refs[&arg.id]))
            }
            ExprKind::Name(_)
                if self.self_expr.is_some()
                    && self.a.refs.get(&arg.id).is_some_and(|&s| self.a.sym(s).rust_name == "self") =>
            {
                "self.type_name()".to_string()
            }
            _ => format!("\"{}\"", object.rust_name),
        }
    }
}

impl Codegen<'_> {
    /// `FillChar(x, count, value)`, in the cases sema allows.
    fn fill_char(&mut self, call_id: NodeId, args: &[Expr]) -> String {
        let (target, count, value) = (&args[0], &args[1], &args[2]);
        // Part of a string or array, from an element: `s[1]`, `a[i]`.
        if let ExprKind::Index(base, indexes) = &target.kind
            && indexes.len() == 1
        {
            let base_ty = self.a.type_of(base.id).clone();
            if matches!(base_ty, Type::Str { .. }) {
                let place = self.place_receiver(base);
                let (start, count) = (self.expr(&indexes[0]).0, self.expr(count).0);
                return format!("{place}.fill({start}, {count}, {});", self.fill_value(value, &Type::Char));
            }
            if let Type::Array(a) = &base_ty {
                let place = self.place_receiver(base);
                let (start, count) = (self.expr(&indexes[0]).0, self.expr(count).0);
                return format!("{place}.fill_from({start}, {count}, {});", self.fill_value(value, &a.elem));
            }
        }
        let ty = self.a.type_of(target.id).clone();
        let zero = self.a.fill_zero.contains(&call_id);
        match &ty {
            Type::Str { .. } => {
                let place = self.place_receiver(target);
                format!("{place}.fill(0, {}, {});", self.expr(count).0, self.fill_value(value, &Type::Char))
            }
            // Filling with zeros is resetting to the default value.
            _ if zero => format!("{} = {};", self.place(target), super::default_value(&ty)),
            Type::Array(a) => {
                let place = self.place_receiver(target);
                format!("{place}.fill({});", self.fill_value(value, &a.elem))
            }
            _ => unreachable!("sema checked FillChar"),
        }
    }

    /// A `FillChar` value as an element of type `elem`: `b' '` for a
    /// `Char`, `value != 0` for a `Boolean`.
    fn fill_value(&mut self, value: &Expr, elem: &Type) -> String {
        match elem {
            Type::Bool => match &value.kind {
                ExprKind::Int(n) => (*n != 0).to_string(),
                _ => format!("{} != 0", paren(self.expr(value), prec::CMP + 1)),
            },
            // A literal number is already a fine `u8`.
            Type::Char if matches!(self.a.type_of(value.id), Type::IntLit(_)) => self.expr(value).0,
            Type::Char if matches!(self.a.type_of(value.id), Type::Int(_)) => {
                format!("{} as u8", paren(self.expr(value), prec::AS))
            }
            _ => self.coerce(value, elem).0,
        }
    }
}
