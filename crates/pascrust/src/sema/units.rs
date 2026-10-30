//! The standard Turbo Pascal units: `Crt`, `Dos`, `Graph` and the parts of `System`
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
    Bool,
    /// A `var` parameter of type `Word`.
    VarWord,
    /// A `var` parameter of type `Integer`.
    VarInteger,
    /// A `var` parameter of type `Text`.
    VarText,
    /// A `var` parameter of a string type at least this long (`FSplit`'s
    /// `var Name: NameStr` is `VarStr(8)`): TP wants the exact type, and a
    /// shorter string couldn't hold the result.
    VarStr(u8),
    /// A `var` parameter of type `Boolean`.
    VarBool,
    /// A `var` parameter of the `Dos` unit's `SearchRec`.
    VarSearchRec,
    /// A `var` parameter of the `Dos` unit's `DateTime`.
    VarDateTime,
    /// A `var` parameter of type `LongInt`.
    VarLongInt,
    /// A file variable of either kind, `Text` or `file of T` (TP's untyped
    /// `var F`).
    VarFile,
    /// `Graph`'s `FillPatternType`, by value and as a `var` parameter.
    FillPattern,
    VarFillPattern,
    /// A `var` parameter of one of the unit's record types, by its
    /// (lowercase) name: `VarRecord("linesettingstype")`.
    VarRecord(&'static str),
    /// TP's untyped `var` parameter: any variable's bytes, as `GetImage`
    /// writes them (`Untyped`) or `PutImage` reads them (`UntypedIn`).
    Untyped,
    UntypedIn,
    /// A `PChar` that's only read: a `PChar`, `nil`, a string constant, or
    /// a zero-based `Char` array (lent a `PChar` for the statement).
    PChar,
    /// A `Pointer`, by value and as a `var` parameter.
    Pointer,
    VarPointer,
    /// A `PChar` the routine writes through (`StrCopy`'s `Dest`).
    PCharDest,
    Char,
    /// `DrawPoly`'s points: TP's untyped `var PolyPoints`, here an array of
    /// `PointType`.
    Points,
}

/// The `Dos` unit's `SearchRec`, filled in by `FindFirst`/`FindNext`. The
/// runtime defines the matching Rust struct. `Name` is a full `string`
/// rather than TP's `string[12]`, so long file names aren't cut off.
pub fn search_rec_type() -> Type {
    use super::types::{ArrayType, Field, RecordType};
    let field = |name: &str, ty: Type| Field { name: name.to_string(), rust_name: name.to_ascii_lowercase(), ty };
    let fill = Type::Array(std::rc::Rc::new(ArrayType {
        index: Type::Int(IntKind::Integer),
        lo: 1,
        hi: 21,
        elem: Type::Int(IntKind::Byte),
        alias: None,
    }));
    Type::Record(std::rc::Rc::new(RecordType {
        rust_name: "SearchRec".to_string(),
        fields: vec![
            field("Fill", fill),
            field("Attr", Type::Int(IntKind::Byte)),
            field("Time", Type::Int(IntKind::LongInt)),
            field("Size", Type::Int(IntKind::LongInt)),
            field("Name", Type::Str { cap: 255 }),
        ],
    }))
}

/// The `Dos` unit's `DateTime`, for `PackTime` and `UnpackTime`.
pub fn date_time_type() -> Type {
    use super::types::{Field, RecordType};
    let names = ["Year", "Month", "Day", "Hour", "Min", "Sec"];
    let fields = names
        .iter()
        .map(|n| Field { name: n.to_string(), rust_name: n.to_ascii_lowercase(), ty: Type::Int(IntKind::Word) })
        .collect();
    Type::Record(std::rc::Rc::new(RecordType { rust_name: "DateTime".to_string(), fields }))
}

/// The `Dos` unit's `Registers`, for `Intr` and `MsDos`, which PascRust
/// refuses: declared so a program that uses them gets that error and not
/// "unknown type". Only the `Word` registers (TP's is a variant record that
/// also has the `Byte` halves, `AL`, `AH`, ...).
pub fn registers_type() -> Type {
    use super::types::{Field, RecordType};
    let names = ["AX", "BX", "CX", "DX", "BP", "SI", "DI", "DS", "ES", "Flags"];
    let fields = names
        .iter()
        .map(|n| Field { name: n.to_string(), rust_name: n.to_ascii_lowercase(), ty: Type::Int(IntKind::Word) })
        .collect();
    Type::Record(std::rc::Rc::new(RecordType { rust_name: "Registers".to_string(), fields }))
}

/// A record type a unit declares, whose Rust struct the runtime defines with
/// the fields in snake case (`XStart` is `x_start`).
fn unit_record(rust_name: &str, fields: &[(&str, Type)]) -> Type {
    use super::types::{Field, RecordType};
    let fields = fields
        .iter()
        .map(|(name, ty)| Field { name: name.to_string(), rust_name: super::names::snake_case(name), ty: ty.clone() })
        .collect();
    Type::Record(std::rc::Rc::new(RecordType { rust_name: rust_name.to_string(), fields }))
}

/// `Graph`'s record types.
/// A record type a unit declares (`Graph`'s and `WinDos`'s), by its
/// lowercase name.
pub fn record_type(name: &str) -> Option<Type> {
    let (int, word) = (Type::Int(IntKind::Integer), Type::Int(IntKind::Word));
    let ints = |names: &[&'static str]| names.iter().map(|&n| (n, int.clone())).collect::<Vec<_>>();
    let words = |names: &[&'static str]| names.iter().map(|&n| (n, word.clone())).collect::<Vec<_>>();
    Some(match name {
        "pointtype" => unit_record("PointType", &ints(&["X", "Y"])),
        "arccoordstype" => unit_record("ArcCoordsType", &ints(&["X", "Y", "XStart", "YStart", "XEnd", "YEnd"])),
        "linesettingstype" => unit_record("LineSettingsType", &words(&["LineStyle", "Pattern", "Thickness"])),
        "fillsettingstype" => unit_record("FillSettingsType", &words(&["Pattern", "Color"])),
        "textsettingstype" => {
            unit_record("TextSettingsType", &words(&["Font", "Direction", "CharSize", "Horiz", "Vert"]))
        }
        "palettetype" => {
            let colors = Type::Array(std::rc::Rc::new(super::types::ArrayType {
                index: Type::Int(IntKind::Integer),
                lo: 0,
                hi: 15,
                elem: Type::Int(IntKind::ShortInt),
                alias: None,
            }));
            unit_record("PaletteType", &[("Size", Type::Int(IntKind::Byte)), ("Colors", colors)])
        }
        "viewporttype" => {
            let mut fields = ints(&["x1", "y1", "x2", "y2"]);
            fields.push(("Clip", Type::Bool));
            unit_record("ViewPortType", &fields)
        }
        // `WinDos`'s `SearchRec`, with the name as a C-style string.
        "tsearchrec" => {
            let array = |lo, hi, elem| {
                Type::Array(std::rc::Rc::new(super::types::ArrayType {
                    index: Type::Int(IntKind::Integer),
                    lo,
                    hi,
                    elem,
                    alias: None,
                }))
            };
            unit_record(
                "TSearchRec",
                &[
                    ("Fill", array(1, 21, Type::Int(IntKind::Byte))),
                    ("Attr", Type::Int(IntKind::Byte)),
                    ("Time", Type::Int(IntKind::LongInt)),
                    ("Size", Type::Int(IntKind::LongInt)),
                    ("Name", array(0, 12, Type::Char)),
                ],
            )
        }
        _ => return None,
    })
}

/// `Graph`'s `FillPatternType = array[1..8] of Byte`: an 8x8 bit pattern.
pub fn fill_pattern_type() -> Type {
    Type::Array(std::rc::Rc::new(super::types::ArrayType {
        index: Type::Int(IntKind::Integer),
        lo: 1,
        hi: 8,
        elem: Type::Int(IntKind::Byte),
        alias: Some("FillPatternType".to_string()),
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ret {
    Bool,
    Char,
    Byte,
    Word,
    Integer,
    LongInt,
    Str,
    PChar,
}

impl Ret {
    pub fn ty(self) -> Type {
        match self {
            Ret::Bool => Type::Bool,
            Ret::Char => Type::Char,
            Ret::Byte => Type::Int(IntKind::Byte),
            Ret::Word => Type::Int(IntKind::Word),
            Ret::Integer => Type::Int(IntKind::Integer),
            Ret::LongInt => Type::Int(IntKind::LongInt),
            Ret::Str => Type::Str { cap: 255 },
            Ret::PChar => Type::PChar,
        }
    }
}

#[derive(Debug)]
pub struct UnitRoutine {
    /// Lowercase Pascal name.
    pub name: &'static str,
    /// For a routine PascRust refuses: why (calling it is an error).
    pub refused: Option<&'static str>,
    /// The runtime function.
    pub rust: &'static str,
    pub params: &'static [Param],
    pub ret: Option<Ret>,
}

/// A shorthand for building the tables below. `$($p),*` repeats for each
/// parameter kind listed in the brackets.
macro_rules! routine {
    // `$p:ident $(($arg:expr))?` takes a parameter kind with an optional
    // argument, like `VarStr(8)`.
    ($name:literal => $rust:literal ($($p:ident $(($arg:expr))?),*) $(-> $ret:ident)?) => {
        UnitRoutine {
            name: $name,
            refused: None,
            rust: $rust,
            params: &[$(Param::$p $(($arg))?),*],
            ret: routine!(@ret $($ret)?),
        }
    };
    // A routine that needs real DOS: declared, so calling it gives a clear
    // error rather than "unknown identifier".
    ($name:literal => refused $why:literal) => {
        UnitRoutine { name: $name, refused: Some($why), rust: "", params: &[], ret: None }
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
    // Faked: there's no 640 KB heap to run out of.
    routine!("memavail" => "mem_avail"() -> LongInt),
    routine!("maxavail" => "max_avail"() -> LongInt),
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
    routine!("assigncrt" => "assign_crt"(VarText)),
];

pub static DOS: &[UnitRoutine] = &[
    routine!("exec" => "exec"(Str, Str)),
    routine!("dosexitcode" => "dos_exit_code"() -> Word),
    routine!("gettime" => "get_time"(VarWord, VarWord, VarWord, VarWord)),
    routine!("getdate" => "get_date"(VarWord, VarWord, VarWord, VarWord)),
    routine!("fsearch" => "f_search"(Str, Str) -> Str),
    routine!("fexpand" => "f_expand"(Str) -> Str),
    routine!("getenv" => "get_env"(Str) -> Str),
    routine!("fsplit" => "f_split"(Str, VarStr(67), VarStr(8), VarStr(4))),
    routine!("findfirst" => "find_first"(Str, Int, VarSearchRec)),
    routine!("findnext" => "find_next"(VarSearchRec)),
    routine!("getftime" => "get_f_time"(VarFile, VarLongInt)),
    routine!("setftime" => "set_f_time"(VarFile, Int)),
    routine!("getfattr" => "get_f_attr"(VarFile, VarWord)),
    routine!("setfattr" => "set_f_attr"(VarFile, Int)),
    routine!("packtime" => "pack_time"(VarDateTime, VarLongInt)),
    routine!("unpacktime" => "unpack_time"(Int, VarDateTime)),
    routine!("envcount" => "env_count"() -> Integer),
    routine!("envstr" => "env_str"(Int) -> Str),
    routine!("dosversion" => "dos_version"() -> Word),
    routine!("disksize" => "disk_size"(Int) -> LongInt),
    routine!("diskfree" => "disk_free"(Int) -> LongInt),
    routine!("setdate" => "set_date"(Int, Int, Int)),
    routine!("settime" => "set_time"(Int, Int, Int, Int)),
    routine!("getcbreak" => "get_cbreak"(VarBool)),
    routine!("setcbreak" => "set_cbreak"(Bool)),
    routine!("getverify" => "get_verify"(VarBool)),
    routine!("setverify" => "set_verify"(Bool)),
    routine!("swapvectors" => "swap_vectors"()),
    routine!("intr" => refused "it calls a DOS or BIOS interrupt, which needs real DOS"),
    routine!("msdos" => refused "it calls DOS itself (interrupt 21h), which needs real DOS"),
    routine!("keep" => refused "it leaves the program resident in DOS memory, which needs real DOS"),
    routine!("getintvec" => "get_int_vec"(Int, VarPointer)),
    routine!("setintvec" => "set_int_vec"(Int, Pointer)),
];

/// A constant a unit exports: Pascal name, Rust name, value.
pub type UnitConst = (&'static str, &'static str, i64);

/// The 16 colours, shared by `Crt` and `Graph`.
pub static COLORS: &[UnitConst] = &[
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
];

/// The `Dos` unit's file attributes, for `FindFirst` and `GetFAttr`.
pub static DOS_CONSTS: &[UnitConst] = &[
    ("readonly", "READ_ONLY", 0x01),
    ("hidden", "HIDDEN", 0x02),
    ("sysfile", "SYS_FILE", 0x04),
    ("volumeid", "VOLUME_ID", 0x08),
    ("directory", "DIRECTORY", 0x10),
    ("archive", "ARCHIVE", 0x20),
    ("anyfile", "ANY_FILE", 0x3F),
];

static DOS_GROUPS: &[&[UnitConst]] = &[DOS_CONSTS];

/// `WinDos` (TP 7): the `Dos` unit with C-style strings (`PChar`), for code
/// shared with Windows programs. The routines without strings are `Dos`'s
/// own; the others wrap theirs. `TDateTime` and `TRegisters` are `Dos`'s
/// `DateTime` and `Registers`. (The `PChar` functions return their first
/// argument, except `GetEnvVar`: see `Checker::pchar_sources`.)
pub static WINDOS: &[UnitRoutine] = &[
    routine!("gettime" => "get_time"(VarWord, VarWord, VarWord, VarWord)),
    routine!("getdate" => "get_date"(VarWord, VarWord, VarWord, VarWord)),
    routine!("settime" => "set_time"(Int, Int, Int, Int)),
    routine!("setdate" => "set_date"(Int, Int, Int)),
    routine!("getftime" => "get_f_time"(VarFile, VarLongInt)),
    routine!("setftime" => "set_f_time"(VarFile, Int)),
    routine!("getfattr" => "get_f_attr"(VarFile, VarWord)),
    routine!("setfattr" => "set_f_attr"(VarFile, Int)),
    routine!("packtime" => "pack_time"(VarDateTime, VarLongInt)),
    routine!("unpacktime" => "unpack_time"(Int, VarDateTime)),
    routine!("dosversion" => "dos_version"() -> Word),
    routine!("disksize" => "disk_size"(Int) -> LongInt),
    routine!("diskfree" => "disk_free"(Int) -> LongInt),
    routine!("getcbreak" => "get_cbreak"(VarBool)),
    routine!("setcbreak" => "set_cbreak"(Bool)),
    routine!("getverify" => "get_verify"(VarBool)),
    routine!("setverify" => "set_verify"(Bool)),
    routine!("getintvec" => "get_int_vec"(Int, VarPointer)),
    routine!("setintvec" => "set_int_vec"(Int, Pointer)),
    routine!("findfirst" => "win_find_first"(PChar, Int, VarRecord("tsearchrec"))),
    routine!("findnext" => "win_find_next"(VarRecord("tsearchrec"))),
    routine!("fileexpand" => "file_expand"(PCharDest, PChar) -> PChar),
    routine!("filesearch" => "file_search"(PCharDest, PChar, PChar) -> PChar),
    routine!("filesplit" => "file_split"(PChar, PCharDest, PCharDest, PCharDest) -> Word),
    routine!("getcurdir" => "get_cur_dir"(PCharDest, Int) -> PChar),
    routine!("setcurdir" => "set_cur_dir"(PChar)),
    routine!("createdir" => "create_dir"(PChar)),
    routine!("removedir" => "remove_dir"(PChar)),
    routine!("getargcount" => "get_arg_count"() -> Integer),
    routine!("getargstr" => "get_arg_str"(PCharDest, Int, Int) -> PChar),
    routine!("getenvvar" => "get_env_var"(PChar) -> PChar),
    routine!("intr" => refused "it calls a DOS or BIOS interrupt, which needs real DOS"),
    routine!("msdos" => refused "it calls DOS itself (interrupt 21h), which needs real DOS"),
];

pub static WINDOS_CONSTS: &[UnitConst] = &[
    ("fareadonly", "FA_READ_ONLY", 0x01),
    ("fahidden", "FA_HIDDEN", 0x02),
    ("fasysfile", "FA_SYS_FILE", 0x04),
    ("favolumeid", "FA_VOLUME_ID", 0x08),
    ("fadirectory", "FA_DIRECTORY", 0x10),
    ("faarchive", "FA_ARCHIVE", 0x20),
    ("faanyfile", "FA_ANY_FILE", 0x3F),
    ("fmclosed", "FM_CLOSED", 0xD7B0),
    ("fminput", "FM_INPUT", 0xD7B1),
    ("fmoutput", "FM_OUTPUT", 0xD7B2),
    ("fminout", "FM_IN_OUT", 0xD7B3),
    ("fspathname", "FS_PATH_NAME", 79),
    ("fsdirectory", "FS_DIRECTORY", 67),
    ("fsfilename", "FS_FILE_NAME", 8),
    ("fsextension", "FS_EXTENSION", 4),
    ("fcextension", "FC_EXTENSION", 0x01),
    ("fcfilename", "FC_FILE_NAME", 0x02),
    ("fcdirectory", "FC_DIRECTORY", 0x04),
    ("fcwildcards", "FC_WILDCARDS", 0x08),
    ("fcarry", "F_CARRY", 0x0001),
    ("fparity", "F_PARITY", 0x0004),
    ("fauxiliary", "F_AUXILIARY", 0x0010),
    ("fzero", "F_ZERO", 0x0040),
    ("fsign", "F_SIGN", 0x0080),
    ("foverflow", "F_OVERFLOW", 0x0800),
];

static WINDOS_GROUPS: &[&[UnitConst]] = &[WINDOS_CONSTS];

pub static CRT_CONSTS: &[UnitConst] = &[
    ("blink", "BLINK", 128),
    ("bw40", "BW40", 0),
    ("co40", "CO40", 1),
    ("bw80", "BW80", 2),
    ("co80", "CO80", 3),
    ("mono", "MONO", 7),
];

pub static GRAPH: &[UnitRoutine] = &[
    routine!("initgraph" => "init_graph"(VarInteger, VarInteger, Str)),
    routine!("detectgraph" => "detect_graph"(VarInteger, VarInteger)),
    routine!("closegraph" => "close_graph"()),
    routine!("restorecrtmode" => "restore_crt_mode"()),
    routine!("setgraphmode" => "set_graph_mode"(Int)),
    routine!("getgraphmode" => "get_graph_mode"() -> Integer),
    routine!("graphresult" => "graph_result"() -> Integer),
    routine!("grapherrormsg" => "graph_error_msg"(Int) -> Str),
    routine!("getmaxx" => "get_max_x"() -> Integer),
    routine!("getmaxy" => "get_max_y"() -> Integer),
    routine!("getmaxcolor" => "get_max_color"() -> Word),
    routine!("getaspectratio" => "get_aspect_ratio"(VarWord, VarWord)),
    routine!("setcolor" => "set_color"(Int)),
    routine!("getcolor" => "get_color"() -> Word),
    routine!("setbkcolor" => "set_bk_color"(Int)),
    routine!("getbkcolor" => "get_bk_color"() -> Word),
    routine!("setpalette" => "set_palette"(Int, Int)),
    routine!("putpixel" => "put_pixel"(Int, Int, Int)),
    routine!("getpixel" => "get_pixel"(Int, Int) -> Word),
    routine!("line" => "line"(Int, Int, Int, Int)),
    routine!("lineto" => "line_to"(Int, Int)),
    routine!("linerel" => "line_rel"(Int, Int)),
    routine!("moveto" => "move_to"(Int, Int)),
    routine!("moverel" => "move_rel"(Int, Int)),
    routine!("getx" => "get_x"() -> Integer),
    routine!("gety" => "get_y"() -> Integer),
    routine!("setlinestyle" => "set_line_style"(Int, Int, Int)),
    routine!("setwritemode" => "set_write_mode"(Int)),
    routine!("rectangle" => "rectangle"(Int, Int, Int, Int)),
    routine!("setfillstyle" => "set_fill_style"(Int, Int)),
    routine!("setfillpattern" => "set_fill_pattern"(FillPattern, Int)),
    routine!("getfillpattern" => "get_fill_pattern"(VarFillPattern)),
    routine!("bar" => "bar"(Int, Int, Int, Int)),
    routine!("bar3d" => "bar3d"(Int, Int, Int, Int, Int, Bool)),
    routine!("circle" => "circle"(Int, Int, Int)),
    routine!("arc" => "arc"(Int, Int, Int, Int, Int)),
    routine!("ellipse" => "ellipse"(Int, Int, Int, Int, Int, Int)),
    routine!("fillellipse" => "fill_ellipse"(Int, Int, Int, Int)),
    routine!("pieslice" => "pie_slice"(Int, Int, Int, Int, Int)),
    routine!("sector" => "sector"(Int, Int, Int, Int, Int, Int)),
    routine!("floodfill" => "flood_fill"(Int, Int, Int)),
    routine!("cleardevice" => "clear_device"()),
    routine!("setviewport" => "set_view_port"(Int, Int, Int, Int, Bool)),
    routine!("clearviewport" => "clear_view_port"()),
    routine!("settextjustify" => "set_text_justify"(Int, Int)),
    routine!("settextstyle" => "set_text_style"(Int, Int, Int)),
    routine!("outtext" => "out_text"(Str)),
    routine!("outtextxy" => "out_text_xy"(Int, Int, Str)),
    routine!("textwidth" => "text_width"(Str) -> Word),
    routine!("textheight" => "text_height"(Str) -> Word),
    routine!("drawpoly" => "draw_poly"(Int, Points)),
    routine!("fillpoly" => "fill_poly"(Int, Points)),
    routine!("getarccoords" => "get_arc_coords"(VarRecord("arccoordstype"))),
    routine!("getlinesettings" => "get_line_settings"(VarRecord("linesettingstype"))),
    routine!("getfillsettings" => "get_fill_settings"(VarRecord("fillsettingstype"))),
    routine!("gettextsettings" => "get_text_settings"(VarRecord("textsettingstype"))),
    routine!("getviewsettings" => "get_view_settings"(VarRecord("viewporttype"))),
    routine!("getpalette" => "get_palette"(VarRecord("palettetype"))),
    routine!("getdefaultpalette" => "get_default_palette"(VarRecord("palettetype"))),
    routine!("setallpalette" => "set_all_palette"(VarRecord("palettetype"))),
    routine!("getpalettesize" => "get_palette_size"() -> Integer),
    routine!("setrgbpalette" => "set_rgb_palette"(Int, Int, Int, Int)),
    routine!("graphdefaults" => "graph_defaults"()),
    routine!("imagesize" => "image_size"(Int, Int, Int, Int) -> Word),
    routine!("getimage" => "get_image"(Int, Int, Int, Int, Untyped)),
    routine!("putimage" => "put_image"(Int, Int, UntypedIn, Int)),
    routine!("setaspectratio" => "set_aspect_ratio"(Int, Int)),
    routine!("setusercharsize" => "set_user_char_size"(Int, Int, Int, Int)),
    routine!("getdrivername" => "get_driver_name"() -> Str),
    routine!("getmodename" => "get_mode_name"(Int) -> Str),
    routine!("getmaxmode" => "get_max_mode"() -> Integer),
    routine!("getmoderange" => "get_mode_range"(Int, VarInteger, VarInteger)),
    routine!("installuserfont" => "install_user_font"(Str) -> Integer),
    routine!("setgraphbufsize" => "set_graph_buf_size"(Int)),
    routine!("setactivepage" => "set_active_page"(Int)),
    routine!("setvisualpage" => "set_visual_page"(Int)),
    routine!("registerbgidriver" => refused "it registers driver code linked in from an .OBJ file; PascRust's one driver is built in, so leave the call out"),
    routine!("registerbgifont" => refused "it registers font code linked in from an .OBJ file; PascRust draws every font with the built-in one, so leave the call out"),
    routine!("installuserdriver" => refused "it loads a .BGI driver file; PascRust's one driver is built in, so leave the call out"),
];

pub static GRAPH_CONSTS: &[UnitConst] = &[
    ("detect", "DETECT", 0),
    ("cga", "CGA", 1),
    ("mcga", "MCGA", 2),
    ("ega", "EGA", 3),
    ("ega64", "EGA64", 4),
    ("egamono", "EGA_MONO", 5),
    ("ibm8514", "IBM8514", 6),
    ("hercmono", "HERC_MONO", 7),
    ("att400", "ATT400", 8),
    ("vga", "VGA", 9),
    ("pc3270", "PC3270", 10),
    ("vgalo", "VGA_LO", 0),
    ("vgamed", "VGA_MED", 1),
    ("vgahi", "VGA_HI", 2),
    ("grok", "GR_OK", 0),
    ("grerror", "GR_ERROR", -11),
    ("solidln", "SOLID_LN", 0),
    ("dottedln", "DOTTED_LN", 1),
    ("centerln", "CENTER_LN", 2),
    ("dashedln", "DASHED_LN", 3),
    ("userbitln", "USER_BIT_LN", 4),
    ("normwidth", "NORM_WIDTH", 1),
    ("thickwidth", "THICK_WIDTH", 3),
    ("emptyfill", "EMPTY_FILL", 0),
    ("solidfill", "SOLID_FILL", 1),
    ("linefill", "LINE_FILL", 2),
    ("ltslashfill", "LT_SLASH_FILL", 3),
    ("slashfill", "SLASH_FILL", 4),
    ("bkslashfill", "BK_SLASH_FILL", 5),
    ("ltbkslashfill", "LT_BK_SLASH_FILL", 6),
    ("hatchfill", "HATCH_FILL", 7),
    ("xhatchfill", "X_HATCH_FILL", 8),
    ("interleavefill", "INTERLEAVE_FILL", 9),
    ("widedotfill", "WIDE_DOT_FILL", 10),
    ("closedotfill", "CLOSE_DOT_FILL", 11),
    ("userfill", "USER_FILL", 12),
    ("lefttext", "LEFT_TEXT", 0),
    ("centertext", "CENTER_TEXT", 1),
    ("righttext", "RIGHT_TEXT", 2),
    ("bottomtext", "BOTTOM_TEXT", 0),
    ("toptext", "TOP_TEXT", 2),
    ("defaultfont", "DEFAULT_FONT", 0),
    ("triplexfont", "TRIPLEX_FONT", 1),
    ("smallfont", "SMALL_FONT", 2),
    ("sansseriffont", "SANS_SERIF_FONT", 3),
    ("gothicfont", "GOTHIC_FONT", 4),
    ("horizdir", "HORIZ_DIR", 0),
    ("vertdir", "VERT_DIR", 1),
    ("copyput", "COPY_PUT", 0),
    ("xorput", "XOR_PUT", 1),
    ("normalput", "NORMAL_PUT", 0),
    ("orput", "OR_PUT", 2),
    ("andput", "AND_PUT", 3),
    ("notput", "NOT_PUT", 4),
    ("maxcolors", "MAX_COLORS", 15),
    ("usercharsize", "USER_CHAR_SIZE", 0),
];

/// The `Strings` unit (TP 7): C-style strings through `PChar`. The
/// functions that return a `PChar` return one into their first argument,
/// except `StrNew` (`Checker::pchar_sources` counts on that).
pub static STRINGS: &[UnitRoutine] = &[
    routine!("strlen" => "str_len"(PChar) -> Word),
    routine!("strend" => "str_end"(PChar) -> PChar),
    routine!("strmove" => "str_move"(PCharDest, PChar, Int) -> PChar),
    routine!("strcopy" => "str_copy"(PCharDest, PChar) -> PChar),
    routine!("strecopy" => "str_e_copy"(PCharDest, PChar) -> PChar),
    routine!("strlcopy" => "str_l_copy"(PCharDest, PChar, Int) -> PChar),
    routine!("strpcopy" => "str_p_copy"(PCharDest, Str) -> PChar),
    routine!("strcat" => "str_cat"(PCharDest, PChar) -> PChar),
    routine!("strlcat" => "str_l_cat"(PCharDest, PChar, Int) -> PChar),
    routine!("strcomp" => "str_comp"(PChar, PChar) -> Integer),
    routine!("stricomp" => "str_i_comp"(PChar, PChar) -> Integer),
    routine!("strlcomp" => "str_l_comp"(PChar, PChar, Int) -> Integer),
    routine!("strlicomp" => "str_l_i_comp"(PChar, PChar, Int) -> Integer),
    routine!("strscan" => "str_scan"(PChar, Char) -> PChar),
    routine!("strrscan" => "str_r_scan"(PChar, Char) -> PChar),
    routine!("strpos" => "str_pos"(PChar, PChar) -> PChar),
    routine!("strupper" => "str_upper"(PCharDest) -> PChar),
    routine!("strlower" => "str_lower"(PCharDest) -> PChar),
    routine!("strpas" => "str_pas"(PChar) -> Str),
    routine!("strnew" => "str_new"(PChar) -> PChar),
    routine!("strdispose" => "str_dispose"(PChar)),
];

/// The `Overlay` unit, faked (see the runtime's `overlay` module): a
/// Rust program has all its code in memory, so there's nothing to load.
pub static OVERLAY: &[UnitRoutine] = &[
    routine!("ovrinit" => "ovr_init"(Str)),
    routine!("ovrinitems" => "ovr_init_ems"()),
    routine!("ovrsetbuf" => "ovr_set_buf"(Int)),
    routine!("ovrgetbuf" => "ovr_get_buf"() -> LongInt),
    routine!("ovrclearbuf" => "ovr_clear_buf"()),
    routine!("ovrsetretry" => "ovr_set_retry"(Int)),
    routine!("ovrgetretry" => "ovr_get_retry"() -> LongInt),
];

/// Integer constants a unit exports (unlike the `Byte`-sized ones above,
/// these may be negative): Pascal name, Rust name, value.
pub fn unit_integers(name: &str) -> &'static [UnitConst] {
    match name {
        "overlay" => &[
            ("ovrok", "OVR_OK", 0),
            ("ovrerror", "OVR_ERROR", -1),
            ("ovrnotfound", "OVR_NOT_FOUND", -2),
            ("ovrnomemory", "OVR_NO_MEMORY", -3),
            ("ovrioerror", "OVR_IO_ERROR", -4),
            ("ovrnoemsdriver", "OVR_NO_EMS_DRIVER", -5),
            ("ovrnoemsmemory", "OVR_NO_EMS_MEMORY", -6),
        ],
        _ => &[],
    }
}

/// A variable a unit exports that lives in the runtime, read through `get`
/// and assigned through `set` (`TextAttr := $1F` is `set_text_attr(0x1F)`).
pub struct UnitVar {
    pub name: &'static str,
    pub ty: Ret,
    pub get: &'static str,
    pub set: &'static str,
}

/// The variables a unit exports.
pub fn unit_vars(name: &str) -> &'static [UnitVar] {
    match name {
        "crt" => &[
            UnitVar { name: "textattr", ty: Ret::Byte, get: "text_attr", set: "set_text_attr" },
            UnitVar { name: "lastmode", ty: Ret::Word, get: "last_mode", set: "set_last_mode" },
            UnitVar { name: "checkbreak", ty: Ret::Bool, get: "check_break", set: "set_check_break" },
            // Faked: remembered, but they change nothing.
            UnitVar { name: "checkeof", ty: Ret::Bool, get: "check_eof", set: "set_check_eof" },
            UnitVar { name: "checksnow", ty: Ret::Bool, get: "check_snow", set: "set_check_snow" },
            UnitVar { name: "directvideo", ty: Ret::Bool, get: "direct_video", set: "set_direct_video" },
        ],
        "dos" | "windos" => &[UnitVar { name: "doserror", ty: Ret::Integer, get: "dos_error", set: "set_dos_error" }],
        "overlay" => &[
            UnitVar { name: "ovrresult", ty: Ret::Integer, get: "ovr_result", set: "set_ovr_result" },
            UnitVar { name: "ovrfilemode", ty: Ret::Byte, get: "ovr_file_mode", set: "set_ovr_file_mode" },
            UnitVar { name: "ovrloadcount", ty: Ret::Word, get: "ovr_load_count", set: "set_ovr_load_count" },
            UnitVar { name: "ovrtrapcount", ty: Ret::Word, get: "ovr_trap_count", set: "set_ovr_trap_count" },
        ],
        _ => &[],
    }
}

/// Boolean constants a unit exports: Pascal name, Rust name, value.
pub fn unit_bools(name: &str) -> &'static [(&'static str, &'static str, bool)] {
    match name {
        "graph" => &[
            ("clipon", "CLIP_ON", true),
            ("clipoff", "CLIP_OFF", false),
            ("topon", "TOP_ON", true),
            ("topoff", "TOP_OFF", false),
        ],
        _ => &[],
    }
}

/// Types a unit exports. The `Dos` unit's path strings are `string[n]`
/// types sized for DOS's path limits.
pub fn unit_types(name: &str) -> Vec<(&'static str, Type)> {
    match name {
        "windos" => vec![
            ("tsearchrec", record_type("tsearchrec").expect("TSearchRec")),
            ("tdatetime", date_time_type()),
            ("tregisters", registers_type()),
        ],
        "dos" => vec![
            ("pathstr", Type::Str { cap: 79 }),
            ("dirstr", Type::Str { cap: 67 }),
            ("namestr", Type::Str { cap: 8 }),
            ("extstr", Type::Str { cap: 4 }),
            ("searchrec", search_rec_type()),
            ("registers", registers_type()),
            ("datetime", date_time_type()),
        ],
        "graph" => {
            let mut types = vec![("fillpatterntype", fill_pattern_type())];
            for name in [
                "pointtype",
                "arccoordstype",
                "linesettingstype",
                "fillsettingstype",
                "textsettingstype",
                "viewporttype",
                "palettetype",
            ] {
                types.push((name, record_type(name).expect("a Graph record")));
            }
            types
        }
        _ => Vec::new(),
    }
}

// A static may refer to other statics (a `const` may not), so the groups
// of constants each unit exports are statics too.
static CRT_GROUPS: &[&[UnitConst]] = &[COLORS, CRT_CONSTS];
static GRAPH_GROUPS: &[&[UnitConst]] = &[COLORS, GRAPH_CONSTS];

/// The routines and constants (in groups) of a unit, by lowercase name.
pub fn unit(name: &str) -> Option<(&'static [UnitRoutine], &'static [&'static [UnitConst]])> {
    match name {
        "crt" => Some((CRT, CRT_GROUPS)),
        "dos" => Some((DOS, DOS_GROUPS)),
        "graph" => Some((GRAPH, GRAPH_GROUPS)),
        // `Printer` only declares the `Lst` text file; nothing to add yet.
        "printer" => Some((&[], &[])),
        "overlay" => Some((OVERLAY, &[])),
        "strings" => Some((STRINGS, &[])),
        "windos" => Some((WINDOS, WINDOS_GROUPS)),
        _ => None,
    }
}
