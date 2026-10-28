//! The procedures and functions that Turbo Pascal's `System` unit provides.
//!
//! These aren't ordinary procedures: many take any number of arguments
//! (`Writeln`), accept several argument types (`Inc` works on integers and
//! chars) or act on variables (`Readln`). So sema checks each one specially,
//! and codegen translates each one specially.

/// Generates the `Builtin` enum and a lookup table from Pascal name to
/// variant, the same trick as the keyword table in `token.rs`.
macro_rules! builtins {
    ($($variant:ident => $name:literal),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Builtin { $($variant),* }

        impl Builtin {
            /// Every builtin with its lowercase Pascal name.
            pub const ALL: &'static [(&'static str, Builtin)] = &[$(($name, Builtin::$variant)),*];
        }
    };
}

builtins! {
    // I/O
    Write => "write", Writeln => "writeln", Read => "read", Readln => "readln",
    // Control flow
    Halt => "halt", Exit => "exit", Break => "break", Continue => "continue",
    // Ordinals
    Ord => "ord", Chr => "chr", Odd => "odd", Succ => "succ", Pred => "pred",
    Inc => "inc", Dec => "dec", Hi => "hi", Lo => "lo",
    // Arithmetic
    Abs => "abs", Sqr => "sqr", Sqrt => "sqrt", Sin => "sin", Cos => "cos",
    ArcTan => "arctan", Ln => "ln", Exp => "exp", Trunc => "trunc",
    Round => "round", Int => "int", Frac => "frac", Pi => "pi",
    Random => "random", Randomize => "randomize",
    // Strings
    Length => "length", Copy => "copy", Pos => "pos", Concat => "concat",
    Delete => "delete", Insert => "insert", UpCase => "upcase", Str => "str",
    Val => "val",
}

impl Builtin {
    /// Builtins that produce a value (functions), as opposed to procedures.
    pub fn is_function(self) -> bool {
        use Builtin::*;
        !matches!(
            self,
            Write
                | Writeln
                | Read
                | Readln
                | Halt
                | Exit
                | Break
                | Continue
                | Inc
                | Dec
                | Randomize
                | Delete
                | Insert
                | Str
                | Val
        )
    }
}
