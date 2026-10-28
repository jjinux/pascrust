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
    let text = String::from_utf8_lossy(s.as_bytes()).trim_start().to_string();
    let leading = s.length() as usize - text.len();
    match T::parse_pascal(&text) {
        Some(v) => {
            *x = v;
            *code = 0;
        }
        None => {
            // Find the first position where the prefix stops being a valid
            // number, for the error code.
            let bad = (1..=text.len())
                .find(|&n| T::parse_pascal(&text[..n]).is_none() && !matches!(&text[..n], "-" | "+" | "$"))
                .unwrap_or(text.len().max(1));
            *code = (leading + bad) as i16;
        }
    }
}

/// Translate a DOS path like `A:\\GAMES\\SCORES.DAT` for this machine:
/// backslashes become slashes, and a drive letter is looked up in the
/// environment variable `PASCRUST_DRIVE_A` (for drive A:, and so on). An
/// unmapped drive means the current directory.
pub fn dos_path(path: &str) -> std::path::PathBuf {
    let mut rest = path;
    let mut base = None;
    if let [drive, b':', ..] = path.as_bytes()
        && drive.is_ascii_alphabetic()
    {
        rest = &path[2..];
        let var = format!("PASCRUST_DRIVE_{}", drive.to_ascii_uppercase() as char);
        base = Some(std::env::var(var).unwrap_or_else(|_| ".".to_string()));
    }
    let rest = rest.replace('\\', "/");
    match base {
        // `A:\X` is absolute on drive A; `A:X` is relative to it. Either way
        // it lives under the drive's directory.
        Some(base) => std::path::Path::new(&base).join(rest.trim_start_matches('/')),
        None => std::path::PathBuf::from(rest),
    }
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
    }

    #[test]
    fn dos_paths() {
        assert_eq!(dos_path("data\\mark.dat"), std::path::PathBuf::from("data/mark.dat"));
        assert_eq!(dos_path("a:\\tictac"), std::path::PathBuf::from("./tictac"));
    }

    #[test]
    fn random_is_deterministic_and_in_range() {
        for _ in 0..1000 {
            let r = random(10);
            assert!((0..10).contains(&r));
        }
    }
}
