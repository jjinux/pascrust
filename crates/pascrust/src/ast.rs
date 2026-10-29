//! The abstract syntax tree: a plain tree of enums and structs that mirrors
//! Pascal's grammar. The parser builds it; sema annotates it through side
//! tables keyed by [`NodeId`]; codegen walks it.
//!
//! Every expression carries a [`NodeId`]. Instead of storing types inside the
//! tree (which would mean `Option<Type>` fields that are `None` until sema
//! runs), sema records facts in maps such as `HashMap<NodeId, Type>`. That
//! keeps the AST immutable after parsing, which plays nicely with Rust's
//! borrowing rules: codegen can hold `&Expr` references and look things up in
//! sema's tables at the same time.

use crate::diag::Span;

/// A unique number for each expression node, handed out by the parser.
pub type NodeId = u32;

/// An identifier as written in the source. Pascal is case-insensitive, so
/// [`Ident::key`] gives the lowercase form used for lookups, while `name`
/// keeps the original spelling for error messages and generated code.
#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn key(&self) -> String {
        self.name.to_ascii_lowercase()
    }
}

/// A whole `program Name; uses ...; <block>.` file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub name: Ident,
    pub uses: Vec<Ident>,
    pub block: Block,
}

/// A Turbo Pascal unit: a separately written module with a public
/// `interface` part (declarations other code can use) and a private
/// `implementation` part.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub name: Ident,
    /// The `interface` keyword: comments before it describe the unit.
    pub interface_keyword: Span,
    pub interface_uses: Vec<Ident>,
    /// Interface declarations. Procedures here are headings only; their
    /// bodies come in the implementation part.
    pub interface: Vec<Decl>,
    pub implementation_uses: Vec<Ident>,
    pub implementation: Vec<Decl>,
    /// The initialization part: statements run once, before the main
    /// program starts. Empty if there is none.
    pub init: Vec<Stmt>,
    /// The unit's final `end` (comments before it end the initialization).
    pub end: Span,
    pub span: Span,
}

impl Unit {
    pub fn uses(&self) -> impl Iterator<Item = &Ident> {
        self.interface_uses.iter().chain(&self.implementation_uses)
    }
}

/// Declarations followed by a `begin ... end` body. Pascal allows the
/// declaration sections (`const`, `type`, `var`, procedures) in any order and
/// any number of times, so they're kept as a list in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub decls: Vec<Decl>,
    pub body: Vec<Stmt>,
    /// The span of the `begin ... end` (or the closing `end` alone).
    pub body_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Const(ConstDecl),
    Type(TypeDecl),
    Var(VarDecl),
    Proc(ProcDecl),
}

/// `const name = value;` or, for a Turbo Pascal *typed constant*,
/// `const name: Type = value;`. Despite the name, a typed constant is really
/// an initialized variable: the program may assign to it.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub value: ConstValue,
}

/// The right-hand side of a constant declaration. Typed constants of array
/// and record types use parenthesized initializers: `(1, 2, 3)` or
/// `(x: 1; y: 2)`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Expr(Expr),
    List(Vec<ConstValue>, Span),
    Record(Vec<(Ident, ConstValue)>, Span),
}

/// `type Name = <type>;`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: Ident,
    pub ty: TypeExpr,
}

/// `var a, b: Type;`
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub names: Vec<Ident>,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// A type name such as `integer` or `GridT`.
    Named(Ident),
    /// `string` or `string[30]`.
    String(Option<Box<Expr>>, Span),
    /// `array[0..9, 'a'..'z'] of T`. Each index is itself a type (usually a
    /// subrange; an enum or `char` name is also allowed).
    Array { indexes: Vec<TypeExpr>, elem: Box<TypeExpr>, span: Span },
    /// `lo..hi`, where both ends are constant expressions.
    Subrange { lo: Box<Expr>, hi: Box<Expr>, span: Span },
    /// `(red, green, blue)`.
    Enum(Vec<Ident>, Span),
    /// `record ... end`.
    Record(Vec<FieldDecl>, Span),
    /// `^Name`. Pascal allows a pointer to a type that's declared later in
    /// the same `type` section, so this holds a name, not a resolved type.
    Pointer(Ident, Span),
    /// `set of T`.
    Set(Box<TypeExpr>, Span),
    /// `file of T`, or plain `file` (untyped).
    File(Option<Box<TypeExpr>>, Span),
    /// A Turbo Pascal 5.5 `object`: a record with procedures attached
    /// ("methods"), optionally inheriting from a parent object type.
    Object { parent: Option<Ident>, fields: Vec<FieldDecl>, methods: Vec<ProcDecl>, span: Span },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named(id) => id.span,
            TypeExpr::String(_, span)
            | TypeExpr::Array { span, .. }
            | TypeExpr::Subrange { span, .. }
            | TypeExpr::Enum(_, span)
            | TypeExpr::Record(_, span)
            | TypeExpr::Pointer(_, span)
            | TypeExpr::Set(_, span)
            | TypeExpr::File(_, span)
            | TypeExpr::Object { span, .. } => *span,
        }
    }
}

/// `a, b: Type` inside a record.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub names: Vec<Ident>,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcKind {
    Procedure,
    Function,
}

/// What a method is declared as. Constructors and destructors are ordinary
/// procedures here, but `New(p, Init)` must name a constructor, as in TP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MethodRole {
    #[default]
    Plain,
    Constructor,
    Destructor,
}

/// A procedure or function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcDecl {
    pub kind: ProcKind,
    /// For a method implementation, `procedure Queue.AddQ(...)`, the object
    /// type it belongs to (`Queue`).
    pub owner: Option<Ident>,
    pub name: Ident,
    pub params: Vec<ParamGroup>,
    /// The result type, for functions.
    pub ret: Option<TypeExpr>,
    pub body: ProcBody,
    pub span: Span,
    /// Declared `virtual` (in an object type): calls choose the version for
    /// the object's real type at run time.
    pub is_virtual: bool,
    /// Declared with `constructor` or `destructor` rather than `procedure`.
    pub role: MethodRole,
    /// The span of the heading alone (`procedure Push(var list: NodePtr; v: integer);`),
    /// which codegen copies into a doc comment.
    pub header_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcBody {
    Block(Box<Block>),
    /// `forward;`: the body comes later in the same scope. Pascal needs this
    /// for mutual recursion because everything must be declared before use.
    Forward,
    /// `external;`: implemented outside Pascal.
    External,
    /// A heading with no body: in a unit's interface, or a method inside an
    /// `object` type. The body comes later.
    Heading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    /// Passed by value: the callee gets its own copy.
    Value,
    /// `var`: passed by reference, so the callee can modify the caller's
    /// variable.
    Var,
    /// `const` (Turbo Pascal): passed however is cheapest, but read-only.
    Const,
}

/// `var a, b: Integer` in a parameter list.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamGroup {
    pub mode: ParamMode,
    pub names: Vec<Ident>,
    /// `None` for an untyped parameter (`var buf`), a TP feature.
    pub ty: Option<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// The empty statement, e.g. between `;` and `end`. Pascal uses `;` to
    /// *separate* statements, not to terminate them, so `begin a; end`
    /// contains two statements: `a` and an empty one.
    Empty,
    Assign {
        target: Expr,
        value: Expr,
    },
    /// A procedure call used as a statement: `writeln('hi')` or `Setup`.
    Call(Expr),
    Compound(Vec<Stmt>),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    /// `repeat ... until cond`. Unlike `while`, the body is a statement list
    /// and always runs at least once.
    Repeat {
        body: Vec<Stmt>,
        cond: Expr,
    },
    For {
        var: Ident,
        start: Expr,
        end: Expr,
        down: bool,
        body: Box<Stmt>,
    },
    Case {
        scrutinee: Expr,
        arms: Vec<CaseArm>,
        else_body: Option<Vec<Stmt>>,
    },
    With {
        records: Vec<Expr>,
        body: Box<Stmt>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseArm {
    pub labels: Vec<CaseLabel>,
    pub body: Stmt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaseLabel {
    Value(Expr),
    Range(Expr, Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Real(f64),
    Str(Vec<u8>),
    Nil,
    /// A bare name: a variable, constant, enum value, parameterless function
    /// call, ... Sema works out which.
    Name(Ident),
    /// `record.field`
    Field(Box<Expr>, Ident),
    /// `array[i, j]`. Pascal's `a[i, j]` is shorthand for `a[i][j]`.
    Index(Box<Expr>, Vec<Expr>),
    /// `pointer^`
    Deref(Box<Expr>),
    /// `f(a, b)`. Also covers type casts like `Integer(c)`, which look the same.
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// A set constructor: `[1, 3..5, x]`.
    Set(Vec<SetElem>),
    /// `@x`, the address of a variable.
    AddrOf(Box<Expr>),
    /// `value:width` or `value:width:decimals`. Only valid as an argument to
    /// `Write`, `Writeln` and `Str`.
    Format {
        value: Box<Expr>,
        width: Box<Expr>,
        decimals: Option<Box<Expr>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetElem {
    Single(Expr),
    Range(Expr, Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Plus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Multiplying operators: highest precedence.
    Mul,
    /// `/` always produces a real, even for two integers.
    RealDiv,
    /// `div`: integer division, truncating toward zero.
    Div,
    Mod,
    And,
    Shl,
    Shr,
    // Adding operators.
    Add,
    Sub,
    Or,
    Xor,
    // Relational operators: lowest precedence. That's why Pascal needs
    // parentheses in `(a > 0) and (b > 0)`: without them `0 and b` binds
    // first.
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    In,
}
