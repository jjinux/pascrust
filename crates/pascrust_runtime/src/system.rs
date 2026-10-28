//! The rest of Turbo Pascal's `System` unit: random numbers and number ↔
//! string conversion.

use std::cell::Cell;

use crate::io::PasWrite;
use crate::string::PString;

// `thread_local!` gives each thread its own copy of a global. Pascal programs
// are single-threaded, and this avoids `static mut` (which needs `unsafe`).
// `Cell` allows changing the value through a shared reference.
thread_local! {
    static RAND_SEED: Cell<u32> = const { Cell::new(0) };
}

/// Turbo Pascal's random number generator: a linear congruential generator.
/// Using the same formula (and the same starting seed, 0) means a program
/// produces the same "random" sequence that it did under TP.
fn next_seed() -> u32 {
    RAND_SEED.with(|seed| {
        let next = seed.get().wrapping_mul(134775813).wrapping_add(1);
        seed.set(next);
        next
    })
}

/// Pascal `Random(n)`: an integer in `0..n`.
pub fn random(n: impl Into<i32>) -> i16 {
    let n = n.into();
    if n <= 0 {
        return 0;
    }
    ((u64::from(next_seed()) * n as u64) >> 32) as i16
}

/// Pascal `Random` without an argument: a real in `[0, 1)`.
pub fn random_real() -> f64 {
    f64::from(next_seed()) / 4294967296.0
}

/// Pascal `Randomize`: seed the generator from the clock.
pub fn randomize() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() ^ d.as_secs() as u32);
    RAND_SEED.with(|seed| seed.set(nanos));
}

/// Pascal `Str(x:w:d, s)`: format a number as `Write` would.
pub fn pas_str(value: impl PasWrite) -> PString {
    let mut out = Vec::new();
    value.pas_write(&mut out);
    PString::from_bytes(&out)
}

/// A type `Val` can parse a string into.
pub trait ValTarget: Sized {
    fn parse_pascal(text: &str) -> Option<Self>;
}

macro_rules! impl_val_int {
    ($($t:ty),*) => {
        $( impl ValTarget for $t {
            fn parse_pascal(text: &str) -> Option<$t> {
                // Turbo Pascal accepts `$FF` hex notation in `Val`.
                match text.strip_prefix('$') {
                    Some(hex) => <$t>::from_str_radix(hex, 16).ok(),
                    None => text.parse().ok(),
                }
            }
        } )*
    };
}
impl_val_int!(i8, i16, i32, u8, u16);

impl ValTarget for f64 {
    fn parse_pascal(text: &str) -> Option<f64> {
        text.parse().ok()
    }
}

/// Pascal `Val(s, x, code)`: parse `s` into `x`. `code` is 0 on success, or
/// the 1-based position of the first bad character.
pub fn val<T: ValTarget>(s: impl Into<PString>, x: &mut T, code: &mut i16) {
    let s = s.into();
    let bytes = s.as_bytes();
    let leading = bytes.iter().take_while(|b| b.is_ascii_whitespace()).count();
    // A number is plain ASCII, so a character from the upper half of code
    // page 437 (an accented letter, say) is where it stops. Up to there the
    // bytes are valid UTF-8, so we can slice them as a `&str`.
    let ascii = bytes[leading..].iter().take_while(|b| b.is_ascii()).count();
    let text = std::str::from_utf8(&bytes[leading..leading + ascii]).expect("ASCII is valid UTF-8");
    let parsed = if leading + ascii == bytes.len() { T::parse_pascal(text) } else { None };
    match parsed {
        Some(v) => {
            *x = v;
            *code = 0;
        }
        None => {
            // Find the first position where the prefix stops being a valid
            // number, for the error code.
            let bad = (1..=text.len())
                .find(|&n| T::parse_pascal(&text[..n]).is_none() && !matches!(&text[..n], "-" | "+" | "$"))
                .unwrap_or(text.len() + 1);
            *code = (leading + bad) as i16;
        }
    }
}

/// Translate a DOS path like `A:\\GAMES\\SCORES.DAT` for this machine:
/// backslashes become slashes, and a drive letter is looked up in the
/// environment variable `PASCRUST_DRIVE_A` (for drive A:, and so on). An
/// unmapped drive is the directory the program started in. DOS file names
/// are case-insensitive, so each part of the path matches an existing file
/// or directory whatever its case.
pub fn dos_path(path: &str) -> std::path::PathBuf {
    let mut rest = path;
    let mut base = None;
    if let [drive, b':', ..] = path.as_bytes()
        && drive.is_ascii_alphabetic()
    {
        rest = &path[2..];
        let var = format!("PASCRUST_DRIVE_{}", drive.to_ascii_uppercase() as char);
        // `join` with an absolute path gives that path, so a relative
        // mapping is relative to where the program started.
        base = Some(start_dir().join(std::env::var(var).unwrap_or_default()));
    }
    let rest = rest.replace('\\', "/");
    let path = match base {
        // `A:\X` is absolute on drive A; `A:X` is relative to it. Either way
        // it lives under the drive's directory.
        Some(base) => base.join(rest.trim_start_matches('/')),
        None => std::path::PathBuf::from(rest),
    };
    match_case(&path)
}

/// The current directory when the program started (the first time this is
/// called, which is before any `ChDir`: see `run`).
pub(crate) fn start_dir() -> &'static std::path::Path {
    // `OnceLock` holds a value that's computed once, the first time it's
    // needed, and then shared for the rest of the program.
    static START: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    START.get_or_init(|| std::env::current_dir().unwrap_or_else(|_| ".".into()))
}

/// Spell each part of `path` the way the file or directory on disk is
/// spelled, if one matches ignoring case. (On a case-sensitive file system,
/// as on Linux, `TICTAC` would otherwise not find `tictac`.)
fn match_case(path: &std::path::Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for part in path.components() {
        let name = part.as_os_str().to_string_lossy();
        let dir = if out.as_os_str().is_empty() { std::path::Path::new(".") } else { &out };
        // `Iterator::find` stops at the first entry whose name matches.
        let found = match part {
            std::path::Component::Normal(_) => std::fs::read_dir(dir).ok().and_then(|entries| {
                entries.flatten().map(|e| e.file_name()).find(|n| n.to_string_lossy().eq_ignore_ascii_case(&name))
            }),
            _ => None, // `/`, `.` and `..` stay as they are
        };
        out.push(found.as_deref().unwrap_or(part.as_os_str()));
    }
    out
}

/// Pascal `ChDir(path)`.
pub fn ch_dir(path: impl Into<PString>) {
    let path = path.into().to_string();
    if std::env::set_current_dir(dos_path(&path)).is_err() {
        crate::rte(3, format!("Path not found: {path}"));
    }
}

/// Pascal `MkDir(path)`.
pub fn mk_dir(path: impl Into<PString>) {
    let path = path.into().to_string();
    if std::fs::create_dir(dos_path(&path)).is_err() {
        crate::rte(5, format!("File access denied: {path}"));
    }
}

/// Pascal `RmDir(path)`.
pub fn rm_dir(path: impl Into<PString>) {
    let path = path.into().to_string();
    if std::fs::remove_dir(dos_path(&path)).is_err() {
        crate::rte(5, format!("File access denied: {path}"));
    }
}

/// Pascal `ParamCount`: the number of command-line arguments.
pub fn param_count() -> i16 {
    std::env::args().count().saturating_sub(1) as i16
}

/// Pascal `ParamStr(i)`: argument `i`; `ParamStr(0)` is the program's path.
pub fn param_str(i: impl Into<i32>) -> PString {
    let i = i.into();
    if i < 0 {
        return PString::new();
    }
    std::env::args().nth(i as usize).map_or_else(PString::new, |arg| PString::from(arg.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn val_parses_and_reports_errors() {
        let (mut x, mut code) = (0i16, 0i16);
        val("123", &mut x, &mut code);
        assert_eq!((x, code), (123, 0));
        val("12a", &mut x, &mut code);
        assert_eq!(code, 3);
        val("$1F", &mut x, &mut code);
        assert_eq!((x, code), (31, 0));
        // A non-ASCII byte (CP437 200) is a bad character, not a crash.
        val(PString::from_bytes(b"12\xc8"), &mut x, &mut code);
        assert_eq!(code, 3);
        val(PString::from_bytes(b"\xc812"), &mut x, &mut code);
        assert_eq!(code, 1);
        val(" 7", &mut x, &mut code);
        assert_eq!((x, code), (7, 0));
    }

    #[test]
    fn dos_paths() {
        assert_eq!(dos_path("data\\mark.dat"), std::path::PathBuf::from("data/mark.dat"));
        // Drive A: isn't mapped (no PASCRUST_DRIVE_A): it's where we started.
        assert_eq!(dos_path("q:\\tictac"), start_dir().join("tictac"));
    }

    #[test]
    fn dos_paths_ignore_case() {
        let dir = std::env::temp_dir().join(format!("pascrust_case_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("TicTac")).unwrap();
        let found = dos_path(&format!("{}\\TICTAC\\new.dat", dir.display()));
        assert_eq!(found, dir.join("TicTac").join("new.dat"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn random_is_deterministic_and_in_range() {
        for _ in 0..1000 {
            let r = random(10);
            assert!((0..10).contains(&r));
        }
    }
}
