//! Turbo Pascal's `Dos` unit: dates and times, running programs, and file
//! name utilities. DOS paths are translated with [`dos_path`].

use std::cell::Cell;

use chrono::{Datelike, Local, Timelike};

use crate::string::PString;
use crate::system::dos_path;

thread_local! {
    static DOS_ERROR: Cell<i16> = const { Cell::new(0) };
    static EXIT_CODE: Cell<u16> = const { Cell::new(0) };
}

/// The `DosError` variable: the result of the last `Exec`, `FindFirst` or
/// `FindNext` (0 = success, 2 = file not found, 18 = no more files).
pub fn dos_error() -> i16 {
    DOS_ERROR.with(Cell::get)
}

/// `DosError := n`.
pub fn set_dos_error(n: impl Into<i32>) {
    DOS_ERROR.with(|e| e.set(n.into() as i16));
}

/// `FSplit(path, dir, name, ext)`: split a path into its directory (up to
/// and including the last `\` or `:`), its name, and its extension (from
/// the last `.` of the name). The parts are cut to `DirStr`, `NameStr` and
/// `ExtStr`'s lengths (67, 8 and 4), as in TP.
pub fn f_split(path: impl Into<PString>, dir: &mut PString, name: &mut PString, ext: &mut PString) {
    let path = path.into();
    let bytes = path.as_bytes();
    let name_start = bytes.iter().rposition(|&b| matches!(b, b'\\' | b':' | b'/')).map_or(0, |i| i + 1);
    let (dir_part, file) = bytes.split_at(name_start);
    let ext_start = file.iter().rposition(|&b| b == b'.').unwrap_or(file.len());
    *dir = PString::from_bytes(dir_part).truncated(67);
    *name = PString::from_bytes(&file[..ext_start]).truncated(8);
    *ext = PString::from_bytes(&file[ext_start..]).truncated(4);
}

/// `EnvCount`: how many environment variables there are.
pub fn env_count() -> i16 {
    std::env::vars_os().count() as i16
}

/// `EnvStr(i)`: environment variable `i` (from 1) as `NAME=value`, or ''.
pub fn env_str(i: impl Into<i32>) -> PString {
    let i = i.into();
    if i < 1 {
        return PString::new();
    }
    std::env::vars_os().nth(i as usize - 1).map_or_else(PString::new, |(name, value)| {
        PString::from(format!("{}={}", name.to_string_lossy(), value.to_string_lossy()).as_str())
    })
}

/// `DosVersion` (faked): DOS 6.22, the last MS-DOS most of these programs
/// knew. The low byte is the major version and the high byte the minor.
pub fn dos_version() -> u16 {
    22 << 8 | 6
}

/// `DiskSize(drive)` (faked): today's disks are far bigger than the 2 GB a
/// `LongInt` holds, so every drive is a 1 GB disk. Drive 0 is the current
/// one, 1 is A:, ...; a drive letter past Z: is -1, as TP's invalid drive.
pub fn disk_size(drive: impl Into<i32>) -> i32 {
    if (0..=26).contains(&drive.into()) { 1 << 30 } else { -1 }
}

/// `DiskFree(drive)` (faked): 512 MB free on every drive. See `disk_size`.
/// (`DiskFree(0) div 1024` works; `DiskFree(0) * 100` overflows a `LongInt`,
/// as it did on a real disk of a few hundred megabytes.)
pub fn disk_free(drive: impl Into<i32>) -> i32 {
    if (0..=26).contains(&drive.into()) { 1 << 29 } else { -1 }
}

/// `SetDate(year, month, day)` and `SetTime(...)` (faked): they would change
/// the computer's clock, which a program shouldn't, so they do nothing.
pub fn set_date(_year: impl Into<i32>, _month: impl Into<i32>, _day: impl Into<i32>) {}

pub fn set_time(_hour: impl Into<i32>, _minute: impl Into<i32>, _second: impl Into<i32>, _sec100: impl Into<i32>) {}

// DOS's Ctrl-Break checking and disk-write verification: settings a
// program can read back, which change nothing here.
thread_local! {
    static CBREAK: Cell<bool> = const { Cell::new(false) };
    static VERIFY: Cell<bool> = const { Cell::new(false) };
}

/// `GetCBreak(b)` and `SetCBreak(b)` (faked).
pub fn get_cbreak(on: &mut bool) {
    *on = CBREAK.with(Cell::get);
}

pub fn set_cbreak(on: bool) {
    CBREAK.with(|c| c.set(on));
}

/// `GetVerify(b)` and `SetVerify(b)` (faked).
pub fn get_verify(on: &mut bool) {
    *on = VERIFY.with(Cell::get);
}

pub fn set_verify(on: bool) {
    VERIFY.with(|v| v.set(on));
}

/// `SwapVectors` (faked): swapped DOS's interrupt vectors around `Exec`.
/// There are none to swap.
pub fn swap_vectors() {}

/// `DosExitCode`: the exit code of the last program run by `Exec`.
pub fn dos_exit_code() -> u16 {
    EXIT_CODE.with(Cell::get)
}

/// `Exec(path, command_line)`: run another program and wait for it.
pub fn exec(path: impl Into<PString>, command_line: impl Into<PString>) {
    let (path, command_line) = (path.into().to_string(), command_line.into().to_string());
    crate::io::flush_stdout();
    let status = std::process::Command::new(dos_path(&path)).args(command_line.split_whitespace()).status();
    match status {
        Ok(status) => {
            DOS_ERROR.with(|e| e.set(0));
            EXIT_CODE.with(|c| c.set(status.code().unwrap_or(0) as u16));
        }
        Err(_) => DOS_ERROR.with(|e| e.set(2)),
    }
}

/// `GetTime(hour, minute, second, sec100)`.
pub fn get_time(hour: &mut u16, minute: &mut u16, second: &mut u16, sec100: &mut u16) {
    let now = Local::now();
    *hour = now.hour() as u16;
    *minute = now.minute() as u16;
    *second = now.second() as u16;
    *sec100 = (now.nanosecond() / 10_000_000).min(99) as u16;
}

/// `GetDate(year, month, day, day_of_week)`; Sunday is day 0.
pub fn get_date(year: &mut u16, month: &mut u16, day: &mut u16, day_of_week: &mut u16) {
    let today = Local::now().date_naive();
    *year = today.year() as u16;
    *month = today.month() as u16;
    *day = today.day() as u16;
    *day_of_week = today.weekday().num_days_from_sunday() as u16;
}

/// `FSearch(name, dir_list)`: look for a file in the current directory, then
/// in each directory of a `;`-separated list. Returns the path found, or an
/// empty string.
pub fn f_search(name: impl Into<PString>, dir_list: impl Into<PString>) -> PString {
    let (name, dir_list) = (name.into().to_string(), dir_list.into().to_string());
    if dos_path(&name).is_file() {
        return PString::from(name.as_str());
    }
    for dir in dir_list.split(';').filter(|d| !d.is_empty()) {
        let dir = dir.trim();
        let separator = if dir.ends_with('\\') || dir.ends_with(':') { "" } else { "\\" };
        let candidate = format!("{dir}{separator}{name}");
        if dos_path(&candidate).is_file() {
            return PString::from(candidate.as_str());
        }
    }
    PString::new()
}

/// `FExpand(path)`: the full DOS path, like `C:\GAMES\SCORES.DAT`.
///
/// pascrust pretends each drive letter is a directory on this machine (see
/// [`dos_path`]) whose current directory is its root, and that the current
/// drive is C:. So `scores.dat` expands to `C:\SCORES.DAT`, which
/// `dos_path` turns back into `scores.dat` in drive C's directory. Programs
/// that pick paths apart looking for `\` and `:` keep working.
pub fn f_expand(path: impl Into<PString>) -> PString {
    let path = path.into().to_string().replace('/', "\\");
    let (drive, rest) = match path.as_bytes() {
        [d, b':', ..] if d.is_ascii_alphabetic() => (d.to_ascii_uppercase() as char, &path[2..]),
        _ => ('C', path.as_str()),
    };
    // Resolve `.` and `..` components.
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split('\\') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    PString::from(format!("{drive}:\\{}", parts.join("\\")).to_uppercase().as_str())
}

/// `GetEnv(name)`.
pub fn get_env(name: impl Into<PString>) -> PString {
    std::env::var(name.into().to_string()).map_or_else(|_| PString::new(), |v| PString::from(v.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fexpand_gives_dos_paths() {
        assert_eq!(f_expand("mydb"), "C:\\MYDB");
        assert_eq!(f_expand("a:games\\x.dat"), "A:\\GAMES\\X.DAT");
        assert_eq!(f_expand("c:\\tp\\..\\bin"), "C:\\BIN");
    }
}

// ----- FindFirst / FindNext ---------------------------------------------------

/// File attributes, for `FindFirst` and `SearchRec.Attr`.
pub const READ_ONLY: u8 = 0x01;
pub const HIDDEN: u8 = 0x02;
pub const SYS_FILE: u8 = 0x04;
pub const VOLUME_ID: u8 = 0x08;
pub const DIRECTORY: u8 = 0x10;
pub const ARCHIVE: u8 = 0x20;
pub const ANY_FILE: u8 = 0x3F;

/// The `Dos` unit's `SearchRec`. TP kept DOS's search state in `Fill`; we
/// keep a handle there to the matches not yet returned (see `SEARCHES`).
/// `name` is a full string, not TP's `string[12]`, so long names fit.
#[derive(Clone, Copy, Debug, Default)]
pub struct SearchRec {
    pub fill: crate::array::PArray<u8, 1, 21>,
    pub attr: u8,
    /// The modification time, packed as `PackTime` does.
    pub time: i32,
    pub size: i32,
    pub name: PString,
}

/// One file `FindFirst` found.
struct Found {
    name: String,
    attr: u8,
    time: i32,
    size: i32,
}

thread_local! {
    /// The matches each search still has to return, by the handle in its
    /// `SearchRec.Fill`. `RefCell` because `FindNext` changes them.
    static SEARCHES: std::cell::RefCell<std::collections::HashMap<u32, std::collections::VecDeque<Found>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    static NEXT_SEARCH: Cell<u32> = const { Cell::new(1) };
}

/// `FindFirst(path, attr, s)`: find the first file matching `path` (DOS
/// wildcards: `*.PAS`, `DATA?.*`) with attributes allowed by `attr`. Plain
/// files are always included; directories only if `attr` has `Directory`,
/// hidden files (names starting with `.`) only if it has `Hidden`. Matches
/// come in name order. `DosError` is 0 if one was found, 18 if none was,
/// 3 if the directory doesn't exist.
pub fn find_first(path: impl Into<PString>, attr: impl Into<i32>, s: &mut SearchRec) {
    let path = path.into().to_string();
    let attr = attr.into() as u8;
    let split = path.rfind(['\\', '/', ':']).map_or(0, |i| i + 1);
    let (dir, pattern) = path.split_at(split);
    let dir_path = if dir.is_empty() { std::path::PathBuf::from(".") } else { dos_path(dir) };
    let Ok(entries) = std::fs::read_dir(&dir_path) else {
        set_dos_error(3);
        return;
    };
    let mut found: Vec<Found> = Vec::new();
    // DOS listed `.` and `..` in subdirectories; `read_dir` doesn't.
    if attr & DIRECTORY != 0 && matches!(pattern, "*.*" | "*") && !dir.is_empty() {
        for name in [".", ".."] {
            found.push(Found { name: name.to_string(), attr: DIRECTORY, time: 0, size: 0 });
        }
    }
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(meta) = entry.metadata() else { continue };
        let mut file_attr = if meta.is_dir() { DIRECTORY } else { ARCHIVE };
        if meta.permissions().readonly() {
            file_attr |= READ_ONLY;
        }
        if name.starts_with('.') {
            file_attr |= HIDDEN;
        }
        let wanted =
            (file_attr & DIRECTORY == 0 || attr & DIRECTORY != 0) && (file_attr & HIDDEN == 0 || attr & HIDDEN != 0);
        if wanted && dos_match(pattern, &name) {
            let time = meta.modified().map(|t| pack_system_time(t)).unwrap_or(0);
            let size = i32::try_from(meta.len()).unwrap_or(i32::MAX);
            found.push(Found { name, attr: file_attr, time, size });
        }
    }
    found.sort_by_key(|f| f.name.to_ascii_lowercase());
    let id = NEXT_SEARCH.with(|n| {
        let id = n.get();
        n.set(id.wrapping_add(1));
        id
    });
    for (i, byte) in id.to_le_bytes().into_iter().enumerate() {
        s.fill[i as i32 + 1] = byte;
    }
    SEARCHES.with_borrow_mut(|searches| searches.insert(id, found.into()));
    find_next(s);
}

/// `FindNext(s)`: the next match of the search `s` started. `DosError` is
/// 18 when there are no more.
pub fn find_next(s: &mut SearchRec) {
    let id = u32::from_le_bytes([s.fill[1], s.fill[2], s.fill[3], s.fill[4]]);
    let next = SEARCHES.with_borrow_mut(|searches| {
        let next = searches.get_mut(&id).and_then(|matches| matches.pop_front());
        if next.is_none() {
            searches.remove(&id);
        }
        next
    });
    match next {
        Some(found) => {
            s.name = PString::from(found.name.as_str());
            s.attr = found.attr;
            s.time = found.time;
            s.size = found.size;
            set_dos_error(0);
        }
        None => set_dos_error(18),
    }
}

/// Does `name` match the DOS wildcard `pattern`? Case doesn't matter, and
/// the name and the extension are matched separately, as DOS did: `*.*`
/// matches every name (`README` too) and `DATA.*` matches `DATA`.
fn dos_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" || pattern == "*.*" {
        return true;
    }
    let split = |s: &str| match s.rfind('.') {
        Some(i) if i > 0 => (s[..i].to_ascii_lowercase(), s[i + 1..].to_ascii_lowercase()),
        _ => (s.to_ascii_lowercase(), String::new()),
    };
    let (pattern_base, pattern_ext) = split(pattern);
    let (name_base, name_ext) = split(name);
    glob(pattern_base.as_bytes(), name_base.as_bytes()) && glob(pattern_ext.as_bytes(), name_ext.as_bytes())
}

/// `*` matches any run of characters, `?` any one.
fn glob(pattern: &[u8], text: &[u8]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (Some(b'*'), _) => glob(&pattern[1..], text) || (!text.is_empty() && glob(pattern, &text[1..])),
        (Some(b'?'), Some(_)) => glob(&pattern[1..], &text[1..]),
        (Some(p), Some(t)) if p == t => glob(&pattern[1..], &text[1..]),
        _ => false,
    }
}

/// A file time as DOS packed it into a `LongInt`: the date in the high
/// word (years since 1980, month, day) and the time in the low word (hour,
/// minute, seconds / 2), in local time.
fn pack_system_time(time: std::time::SystemTime) -> i32 {
    let local: chrono::DateTime<Local> = time.into();
    let year = (local.year() - 1980).clamp(0, 127) as u32;
    let date = year << 9 | local.month() << 5 | local.day();
    let clock = local.hour() << 11 | local.minute() << 5 | local.second() / 2;
    (date << 16 | clock) as i32
}

#[cfg(test)]
mod find_tests {
    use super::*;

    #[test]
    fn dos_wildcards() {
        assert!(dos_match("*.PAS", "life.pas"));
        assert!(!dos_match("*.PAS", "life.pa"));
        assert!(dos_match("*.*", "README"));
        assert!(dos_match("DATA.*", "data"));
        assert!(dos_match("DATA?.DAT", "data1.dat"));
        assert!(!dos_match("DATA?.DAT", "data12.dat"));
        assert!(dos_match("*", ".hidden"));
    }
}
