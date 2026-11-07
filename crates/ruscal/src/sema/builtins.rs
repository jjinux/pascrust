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
    RunError => "runerror",
    // Ordinals
    Ord => "ord", Chr => "chr", Odd => "odd", Succ => "succ", Pred => "pred",
    Inc => "inc", Dec => "dec", Hi => "hi", Lo => "lo", High => "high", Low => "low",
    Swap => "swap", SizeOf => "sizeof", FillChar => "fillchar", Move => "move",
    // Arithmetic
    Abs => "abs", Sqr => "sqr", Sqrt => "sqrt", Sin => "sin", Cos => "cos",
    ArcTan => "arctan", Ln => "ln", Exp => "exp", Trunc => "trunc",
    Round => "round", Int => "int", Frac => "frac", Pi => "pi",
    Random => "random", Randomize => "randomize",
    // Strings
    Length => "length", Copy => "copy", Pos => "pos", Concat => "concat",
    Delete => "delete", Insert => "insert", UpCase => "upcase", Str => "str",
    Val => "val",
    // Pointers
    New => "new", Dispose => "dispose", Assigned => "assigned", Mark => "mark", Release => "release",
    GetMem => "getmem", FreeMem => "freemem",
    // Sets
    Include => "include", Exclude => "exclude",
    // Files
    Assign => "assign", Reset => "reset", Rewrite => "rewrite", Append => "append",
    Close => "close", Erase => "erase", Rename => "rename", Flush => "flush",
    Truncate => "truncate", Seek => "seek", FilePos => "filepos", FileSize => "filesize",
    BlockRead => "blockread", BlockWrite => "blockwrite",
    Eof => "eof", Eoln => "eoln", SeekEof => "seekeof", SeekEoln => "seekeoln",
    IOResult => "ioresult", TypeOf => "typeof", GetDir => "getdir", SetTextBuf => "settextbuf",
}

impl Builtin {
    /// Builtins that do file or console I/O, which `{$I-}` makes report
    /// errors through `IOResult`.
    pub fn is_io(self) -> bool {
        use Builtin::*;
        matches!(
            self,
            Write
                | Writeln
                | Read
                | Readln
                | Reset
                | Rewrite
                | BlockRead
                | BlockWrite
                | Append
                | Close
                | Erase
                | Rename
                | Flush
                | Truncate
                | Seek
                | FilePos
                | FileSize
                | Eof
                | Eoln
                | SeekEof
                | SeekEoln
        )
    }

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
                | RunError
                | FillChar
                | Move
                | Mark
                | Release
                | GetDir
                | SetTextBuf
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
                | New
                | Dispose
                | Include
                | Exclude
                | Assign
                | Reset
                | Rewrite
                | Append
                | Close
                | Erase
                | Rename
                | Flush
                | Truncate
                | Seek
        )
    }
}
