//! The standard Turbo Pascal units: `Crt`, `Dos` and the parts of `System`
//! that behave like ordinary procedures.
//!
//! Unlike `Writeln` and friends (see `builtins.rs`), these routines have
//! fixed signatures, so they're described by a table: the Pascal name, the
//! Rust function in `pascrust_runtime` that implements it, and the kinds of
//! its parameters. Sema checks calls against the table and codegen emits a
//! plain function call, with no special cases.

use super::types::{IntKind, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    /// Any integer. Runtime functions take `impl Into<i32>`, so any integer
    /// type can be passed without a cast.
    Int,
    Real,
    /// Anything string-like (`impl Into<PString>` in the runtime).
    Str,
    /// A `var` parameter of type `Word`.
    VarWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ret {
    Bool,
    Char,
    Byte,
    Word,
    Integer,
    Str,
}

impl Ret {
    pub fn ty(self) -> Type {
        match self {
            Ret::Bool => Type::Bool,
            Ret::Char => Type::Char,
            Ret::Byte => Type::Int(IntKind::Byte),
            Ret::Word => Type::Int(IntKind::Word),
            Ret::Integer => Type::Int(IntKind::Integer),
            Ret::Str => Type::Str { cap: 255 },
        }
    }
}

#[derive(Debug)]
pub struct UnitRoutine {
    /// Lowercase Pascal name.
    pub name: &'static str,
    /// The runtime function.
    pub rust: &'static str,
    pub params: &'static [Param],
    pub ret: Option<Ret>,
}

/// A shorthand for building the tables below. `$($p),*` repeats for each
/// parameter kind listed in the brackets.
macro_rules! routine {
    ($name:literal => $rust:literal ($($p:ident),*) $(-> $ret:ident)?) => {
        UnitRoutine { name: $name, rust: $rust, params: &[$(Param::$p),*], ret: routine!(@ret $($ret)?) }
    };
    (@ret) => { None };
    (@ret $ret:ident) => { Some(Ret::$ret) };
}

/// `System` routines that are ordinary procedures. Always available.
pub static SYSTEM: &[UnitRoutine] = &[
    routine!("chdir" => "ch_dir"(Str)),
    routine!("mkdir" => "mk_dir"(Str)),
    routine!("rmdir" => "rm_dir"(Str)),
    routine!("paramcount" => "param_count"() -> Integer),
    routine!("paramstr" => "param_str"(Int) -> Str),
];

pub static CRT: &[UnitRoutine] = &[
    routine!("clrscr" => "clr_scr"()),
    routine!("clreol" => "clr_eol"()),
    routine!("gotoxy" => "goto_xy"(Int, Int)),
    routine!("wherex" => "where_x"() -> Byte),
    routine!("wherey" => "where_y"() -> Byte),
    routine!("textcolor" => "text_color"(Int)),
    routine!("textbackground" => "text_background"(Int)),
    routine!("highvideo" => "high_video"()),
    routine!("lowvideo" => "low_video"()),
    routine!("normvideo" => "norm_video"()),
    routine!("window" => "window"(Int, Int, Int, Int)),
    routine!("insline" => "ins_line"()),
    routine!("delline" => "del_line"()),
    routine!("delay" => "delay"(Int)),
    routine!("sound" => "sound"(Int)),
    routine!("nosound" => "no_sound"()),
    routine!("keypressed" => "key_pressed"() -> Bool),
    routine!("readkey" => "read_key"() -> Char),
    routine!("textmode" => "text_mode"(Int)),
    // These are variables in Turbo Pascal; programs only ever read them, so
    // they're functions here.
    routine!("windmin" => "wind_min"() -> Word),
    routine!("windmax" => "wind_max"() -> Word),
    routine!("textattr" => "text_attr"() -> Byte),
];

pub static DOS: &[UnitRoutine] = &[
    routine!("exec" => "exec"(Str, Str)),
    routine!("doserror" => "dos_error"() -> Integer),
    routine!("dosexitcode" => "dos_exit_code"() -> Word),
    routine!("gettime" => "get_time"(VarWord, VarWord, VarWord, VarWord)),
    routine!("getdate" => "get_date"(VarWord, VarWord, VarWord, VarWord)),
    routine!("fsearch" => "f_search"(Str, Str) -> Str),
    routine!("fexpand" => "f_expand"(Str) -> Str),
    routine!("getenv" => "get_env"(Str) -> Str),
];

/// A constant a unit exports: Pascal name, Rust name, value.
pub type UnitConst = (&'static str, &'static str, i64);

pub static CRT_CONSTS: &[UnitConst] = &[
    ("black", "BLACK", 0),
    ("blue", "BLUE", 1),
    ("green", "GREEN", 2),
    ("cyan", "CYAN", 3),
    ("red", "RED", 4),
    ("magenta", "MAGENTA", 5),
    ("brown", "BROWN", 6),
    ("lightgray", "LIGHT_GRAY", 7),
    ("darkgray", "DARK_GRAY", 8),
    ("lightblue", "LIGHT_BLUE", 9),
    ("lightgreen", "LIGHT_GREEN", 10),
    ("lightcyan", "LIGHT_CYAN", 11),
    ("lightred", "LIGHT_RED", 12),
    ("lightmagenta", "LIGHT_MAGENTA", 13),
    ("yellow", "YELLOW", 14),
    ("white", "WHITE", 15),
    ("blink", "BLINK", 128),
    ("bw40", "BW40", 0),
    ("co40", "CO40", 1),
    ("bw80", "BW80", 2),
    ("co80", "CO80", 3),
    ("mono", "MONO", 7),
];

/// Types a unit exports. The `Dos` unit's path strings are `string[n]`
/// types sized for DOS's path limits.
pub fn unit_types(name: &str) -> Vec<(&'static str, Type)> {
    match name {
        "dos" => vec![
            ("pathstr", Type::Str { cap: 79 }),
            ("dirstr", Type::Str { cap: 67 }),
            ("namestr", Type::Str { cap: 8 }),
            ("extstr", Type::Str { cap: 4 }),
        ],
        _ => Vec::new(),
    }
}

/// The routines and constants of a unit, by lowercase name.
pub fn unit(name: &str) -> Option<(&'static [UnitRoutine], &'static [UnitConst])> {
    match name {
        "crt" => Some((CRT, CRT_CONSTS)),
        "dos" => Some((DOS, &[])),
        // `Printer` only declares the `Lst` text file; nothing to add yet.
        "printer" => Some((&[], &[])),
        _ => None,
    }
}
