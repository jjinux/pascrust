//! Turbo Pascal's `Dos` unit: dates and times, running programs, and file
//! name utilities. DOS paths are translated with [`dos_path`].

use std::cell::Cell;

use chrono::{Datelike, Local, TimeZone, Timelike};

use crate::ptr::AnyPtr;
use crate::string::PString;
use crate::system::dos_path;

thread_local! {
    static DOS_ERROR: Cell<i16> = const { Cell::new(0) };
    static EXIT_CODE: Cell<u16> = const { Cell::new(0) };
}

/// The `DosError` variable: the result of the last `Exec`, `FindFirst`,
/// `FindNext` or file time or attribute routine (0 = success, 2 = file not
/// found, 3 = path not found, 18 = no more files).
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
/// Drive C: is the file system's root (or wherever `PASCRUST_DRIVE_C` says,
/// see [`dos_path`]), and it's the current drive. A relative path starts
/// from the drive's current directory (`GetDir`), so `scores.dat` expands
/// to the full path, `C:\USERS\JJ\GAMES\SCORES.DAT`, which `dos_path`
/// turns back into the file. Programs that pick paths apart looking for `\`
/// and `:` keep working.
pub fn f_expand(path: impl Into<PString>) -> PString {
    let path = path.into().to_string().replace('/', "\\");
    let (drive, rest) = match path.as_bytes() {
        [d, b':', ..] if d.is_ascii_alphabetic() => (d.to_ascii_uppercase() as char, &path[2..]),
        _ => ('C', path.as_str()),
    };
    let mut parts: Vec<String> = Vec::new();
    if !rest.starts_with('\\') {
        parts = crate::system::drive_dir_parts(drive);
    }
    // Resolve `.` and `..` components.
    for part in rest.split('\\') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part.to_string()),
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
        // A relative name expands from the current directory, on C:.
        let expanded = f_expand("mydb").to_string();
        assert!(expanded.starts_with("C:\\") && expanded.ends_with("\\MYDB"), "{expanded}");
        assert_eq!(f_expand("a:games\\x.dat"), "A:\\GAMES\\X.DAT");
        assert_eq!(f_expand("c:\\tp\\..\\bin"), "C:\\BIN");
    }
}

/// The `Registers` record of `Intr` and `MsDos`, which PascRust can't run.
/// It exists so programs that declare one still compile up to those calls.
#[derive(Clone, Copy, Debug, Default)]
pub struct Registers {
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,
    pub bp: u16,
    pub si: u16,
    pub di: u16,
    pub ds: u16,
    pub es: u16,
    pub flags: u16,
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
/// keep a handle there to the search's matches, and how many it has
/// returned (see `SEARCHES`).
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

/// How many searches stay open at once. DOS kept a search's state in the
/// `SearchRec` itself, so a program never closes one (and a copy of a
/// `SearchRec` can carry on with it any time). So PascRust keeps every
/// search, up to this many, and then forgets the one used longest ago. A
/// recursive directory walk leaves each level's search idle while it
/// searches the levels below, so the limit is high.
const OPEN_SEARCHES: usize = 10_000;

/// Searches that have run out are rarely used again (only by a copy of a
/// `SearchRec` made before the end), but they can be long lists, so only
/// this many of them are kept, the ones used most recently.
const RUN_OUT_SEARCHES: usize = 100;

/// One `FindFirst`'s matches, when it was last used, and whether
/// `FindNext` has gone past its last match.
struct Search {
    last_used: u64,
    run_out: bool,
    matches: std::rc::Rc<Vec<Found>>,
}

thread_local! {
    /// The open searches, by the handle in bytes 1-4 of a `SearchRec.Fill`;
    /// bytes 5-8 hold how many it has returned. So a copy of a `SearchRec`
    /// goes on from the same place on its own, as in DOS. (`Rc` because
    /// `find_next` holds on to the list while it fills in the `SearchRec`.)
    static SEARCHES: std::cell::RefCell<std::collections::HashMap<u32, Search>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// Counts `FindFirst` and `FindNext` calls: new handles, and the
    /// `last_used` times.
    static CLOCK: Cell<u64> = const { Cell::new(1) };
}

fn tick() -> u64 {
    CLOCK.with(|clock| {
        let now = clock.get();
        clock.set(now + 1);
        now
    })
}

/// The handle and position kept in a `SearchRec`'s `Fill`.
fn search_state(s: &SearchRec) -> (u32, u32) {
    let byte = |i: i32| s.fill[i];
    let handle = u32::from_le_bytes([byte(1), byte(2), byte(3), byte(4)]);
    let position = u32::from_le_bytes([byte(5), byte(6), byte(7), byte(8)]);
    (handle, position)
}

fn set_search_state(s: &mut SearchRec, handle: u32, position: u32) {
    let bytes = handle.to_le_bytes().into_iter().chain(position.to_le_bytes());
    for (i, byte) in bytes.enumerate() {
        s.fill[i as i32 + 1] = byte;
    }
}

/// `FindFirst(path, attr, s)`: find the first file matching `path` (DOS
/// wildcards: `*.PAS`, `DATA?.*`) with attributes allowed by `attr`. Plain
/// files are always included; directories only if `attr` has `Directory`,
/// hidden files (names starting with `.`) only if it has `Hidden`. Matches
/// come in name order. `DosError` is 0 if one was found, 18 if none was,
/// 3 if the directory doesn't exist. `attr = VolumeID` alone asks for the
/// disk's label, which there isn't one of.
pub fn find_first(path: impl Into<PString>, attr: impl Into<i32>, s: &mut SearchRec) {
    let path = path.into().to_string();
    let attr = attr.into() as u8;
    // A new search replaces the one this `SearchRec` held, even if it fails.
    set_search_state(s, 0, 0);
    let split = path.rfind(['\\', '/', ':']).map_or(0, |i| i + 1);
    let (dir, pattern) = path.split_at(split);
    let dir_path = if dir.is_empty() { std::path::PathBuf::from(".") } else { dos_path(dir) };
    let Ok(entries) = std::fs::read_dir(&dir_path) else {
        set_dos_error(3);
        return;
    };
    let mut found: Vec<Found> = Vec::new();
    // DOS listed `.` and `..` in every directory but a root; `read_dir`
    // doesn't list them.
    // `\` is the root, and so is a drive's own directory, however it's
    // reached (`A:\`, `A:\SUB\..`, or `A:` when that's the drive's current
    // directory).
    let canonical = dir_path.canonicalize().ok();
    let drive_root = |drive: &str| dos_path(&format!("{drive}\\")).canonicalize().ok();
    let is_root = (!dir.is_empty() && dir.trim_end_matches(['\\', '/']).is_empty())
        || canonical.as_ref().is_some_and(|p| p.parent().is_none())
        || (dir.get(1..2) == Some(":") && canonical.is_some() && canonical == drive_root(&dir[..2]));
    if attr & DIRECTORY != 0 && !is_root {
        for name in [".", ".."] {
            if dos_match(pattern, name) {
                found.push(Found { name: name.to_string(), attr: DIRECTORY, time: 0, size: 0 });
            }
        }
    }
    for entry in entries.flatten() {
        if attr == VOLUME_ID {
            break;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        // Follow a symbolic link to see what it points to (a link to a
        // directory is a directory); a broken link is listed as itself.
        let Ok(meta) = std::fs::metadata(entry.path()).or_else(|_| entry.metadata()) else { continue };
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
            let time = meta.modified().map(pack_system_time).unwrap_or(0);
            let size = i32::try_from(meta.len()).unwrap_or(i32::MAX);
            found.push(Found { name, attr: file_attr, time, size });
        }
    }
    found.sort_by_key(|f| f.name.to_ascii_lowercase());
    let now = tick();
    // Handles are 32 bits, in `Fill`; four billion searches is plenty.
    let id = now as u32;
    SEARCHES.with_borrow_mut(|searches| {
        searches.insert(id, Search { last_used: now, run_out: false, matches: std::rc::Rc::new(found) });
        let run_out = searches.values().filter(|search| search.run_out).count();
        let forget = if run_out > RUN_OUT_SEARCHES {
            searches.iter().filter(|(_, search)| search.run_out).min_by_key(|(_, search)| search.last_used)
        } else if searches.len() > OPEN_SEARCHES {
            searches.iter().min_by_key(|(_, search)| search.last_used)
        } else {
            None
        };
        if let Some(id) = forget.map(|(&id, _)| id) {
            searches.remove(&id);
        }
    });
    set_search_state(s, id, 0);
    find_next(s);
}

/// `FindNext(s)`: the next match of the search `s` started. `DosError` is
/// 18 when there are no more.
pub fn find_next(s: &mut SearchRec) {
    let (id, position) = search_state(s);
    let matches = SEARCHES.with_borrow_mut(|searches| {
        let search = searches.get_mut(&id)?;
        search.last_used = tick();
        search.run_out = position as usize >= search.matches.len();
        Some(search.matches.clone())
    });
    let Some(found) = matches.as_ref().and_then(|m| m.get(position as usize)) else {
        set_dos_error(18);
        return;
    };
    s.name = PString::from(found.name.as_str());
    s.attr = found.attr;
    s.time = found.time;
    s.size = found.size;
    set_search_state(s, id, position + 1);
    set_dos_error(0);
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
        // DOS padded names with blanks, so `?` also matches nothing at the
        // end: `DATA?.DAT` matches `DATA.DAT`.
        (Some(b'?'), None) => glob(&pattern[1..], text),
        (Some(p), Some(t)) if p == t => glob(&pattern[1..], &text[1..]),
        _ => false,
    }
}

/// A file time as DOS packed it into a `LongInt`: the date in the high
/// word (years since 1980, month, day) and the time in the low word (hour,
/// minute, seconds / 2), in local time.
fn pack_system_time(time: std::time::SystemTime) -> i32 {
    let local: chrono::DateTime<Local> = time.into();
    let mut dt = DateTime {
        year: local.year().clamp(1980, 2107) as u16,
        month: local.month() as u16,
        day: local.day() as u16,
        hour: local.hour() as u16,
        min: local.minute() as u16,
        sec: local.second() as u16,
    };
    let mut packed = 0;
    pack_time(&mut dt, &mut packed);
    packed
}

// ----- File times and attributes -----------------------------------------------

/// The `Dos` unit's `DateTime`, for `PackTime` and `UnpackTime`.
#[derive(Clone, Copy, Debug, Default)]
pub struct DateTime {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub min: u16,
    pub sec: u16,
}

/// `PackTime(dt, time)`: pack a date and time into a `LongInt` as DOS
/// stored file times (see `pack_system_time`). Seconds are kept to 2.
/// Like TP, it adds the fields up in place without checking them, so an
/// out-of-range one spills into the next (day 32 is the next month's 0);
/// but the date and the time are separate 16-bit words, so the time can't
/// spill into the date.
/// `dt` is `&mut` only because TP declares it `var T: DateTime` (passed by
/// reference to save copying it); it isn't changed.
pub fn pack_time(dt: &mut DateTime, time: &mut i32) {
    let date = dt.year.wrapping_sub(1980).wrapping_shl(9).wrapping_add(dt.month.wrapping_shl(5)).wrapping_add(dt.day);
    let clock = dt.hour.wrapping_shl(11).wrapping_add(dt.min.wrapping_shl(5)).wrapping_add(dt.sec / 2);
    *time = ((u32::from(date) << 16) | u32::from(clock)) as i32;
}

/// `UnpackTime(time, dt)`: the reverse of `PackTime`.
pub fn unpack_time(time: impl Into<i32>, dt: &mut DateTime) {
    let time = time.into() as u32;
    let (date, clock) = (time >> 16, time & 0xFFFF);
    *dt = DateTime {
        year: (1980 + (date >> 9)) as u16,
        month: ((date >> 5) & 15) as u16,
        day: (date & 31) as u16,
        hour: (clock >> 11) as u16,
        min: ((clock >> 5) & 63) as u16,
        sec: ((clock & 31) * 2) as u16,
    };
}

/// The file a file variable is assigned to, or `None` (and `DosError` 6,
/// DOS's "invalid handle") if it isn't assigned.
fn file_path(f: &impl crate::io::AnyFile) -> Option<std::path::PathBuf> {
    match f.assigned_name() {
        Some(name) => Some(dos_path(&name.to_string())),
        None => {
            set_dos_error(6);
            None
        }
    }
}

/// `DosError` for an I/O error on `path`: 2 file not found, 3 path not
/// found (its directory is missing), 5 access denied.
fn io_dos_error(e: &std::io::Error, path: &std::path::Path) -> i16 {
    match e.kind() {
        std::io::ErrorKind::NotFound => match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() && !dir.is_dir() => 3,
            _ => 2,
        },
        std::io::ErrorKind::PermissionDenied => 5,
        _ => 1,
    }
}

/// `GetFTime(f, time)`: the file's modification time, packed. (The file
/// routines take `&mut` files, mirroring TP's `var F`; only `SetFTime`
/// changes anything, by flushing.)
pub fn get_f_time(f: &mut impl crate::io::AnyFile, time: &mut i32) {
    let Some(path) = file_path(f) else { return };
    match std::fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(modified) => {
            *time = pack_system_time(modified);
            set_dos_error(0);
        }
        Err(e) => set_dos_error(io_dos_error(&e, &path)),
    }
}

/// `SetFTime(f, time)`: set the file's modification time from a packed
/// time. In TP the file was open, and whatever the program had written
/// before still counts as written before the new time, so it's flushed
/// first. (Writing to the file afterwards changes the time again.)
pub fn set_f_time(f: &mut impl crate::io::AnyFile, time: impl Into<i32>) {
    let Some(path) = file_path(f) else { return };
    f.flush_buffer();
    let mut dt = DateTime::default();
    unpack_time(time, &mut dt);
    let modified = local_time(&dt).into();
    // Setting the time doesn't need write access (it needs to own the
    // file), so a read-only file works too. Windows wants write access.
    let result = std::fs::File::open(&path)
        .and_then(|file| file.set_modified(modified))
        .or_else(|_| std::fs::File::options().write(true).open(&path)?.set_modified(modified));
    if result.is_ok() {
        remember_time(&path);
    }
    set_dos_error(match result {
        Ok(()) => 0,
        Err(e) => io_dos_error(&e, &path),
    });
}

/// A `DateTime` as a local time. DOS stored whatever it was given; a time
/// that doesn't exist (in the hour skipped when clocks go forward, or day
/// 0) is moved to the nearest one that does.
fn local_time(dt: &DateTime) -> chrono::DateTime<Local> {
    let year = i32::from(dt.year);
    let month = u32::from(dt.month).clamp(1, 12);
    // The last valid day of the month, at most: `from_ymd_opt` checks.
    let date = (1..=u32::from(dt.day).clamp(1, 31))
        .rev()
        .find_map(|day| chrono::NaiveDate::from_ymd_opt(year, month, day))
        .unwrap_or_default();
    let clock = chrono::NaiveTime::from_hms_opt(
        u32::from(dt.hour).min(23),
        u32::from(dt.min).min(59),
        u32::from(dt.sec).min(59),
    )
    .unwrap_or_default();
    let naive = date.and_time(clock);
    Local
        .from_local_datetime(&naive)
        .earliest()
        .or_else(|| Local.from_local_datetime(&(naive + chrono::TimeDelta::hours(1))).earliest())
        .unwrap_or_else(|| Local.from_utc_datetime(&naive))
}

/// Which file a path names, however it's reached. On Unix it's the
/// device and inode, so a remembered attribute follows the file through
/// `Rename`, and a new file with the old name doesn't inherit it.
/// Elsewhere it's the full path. (`#[cfg]` picks one definition of each
/// at compile time.)
#[cfg(unix)]
type FileId = (u64, u64);
#[cfg(not(unix))]
type FileId = std::path::PathBuf;

#[cfg(unix)]
fn file_id(_path: &std::path::Path, meta: &std::fs::Metadata) -> FileId {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
}

#[cfg(not(unix))]
fn file_id(path: &std::path::Path, _meta: &std::fs::Metadata) -> FileId {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// The attribute bits a modern file system has no place for (`Hidden`,
/// `SysFile`, `Archive` set by `SetFAttr`), and the file's times then: a
/// different creation time means it's a new file that reused the inode, a
/// different modification time that it's been written since.
struct ExtraAttrs {
    bits: u8,
    created: Option<std::time::SystemTime>,
    modified: Option<std::time::SystemTime>,
}

thread_local! {
    /// The bits `SetFAttr` set, by `file_id`.
    static EXTRA_ATTRS: std::cell::RefCell<std::collections::HashMap<FileId, ExtraAttrs>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// After `SetFTime`: the new time isn't a change to the file's contents.
fn remember_time(path: &std::path::Path) {
    let Ok(meta) = std::fs::metadata(path) else { return };
    EXTRA_ATTRS.with_borrow_mut(|attrs| {
        if let Some(extra) = attrs.get_mut(&file_id(path, &meta)) {
            extra.modified = meta.modified().ok();
        }
    });
}

/// Make the file read-only, or writable again. On Unix, `set_readonly`
/// would make a file writable by everyone; this gives write access back to
/// the owner only.
fn set_read_only(path: &std::path::Path, meta: &std::fs::Metadata, read_only: bool) -> std::io::Result<()> {
    let mut permissions = meta.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = permissions.mode();
        let mode = if read_only {
            mode & !0o222
        } else if permissions.readonly() {
            mode | 0o200
        } else {
            mode
        };
        permissions.set_mode(mode);
    }
    #[cfg(not(unix))]
    permissions.set_readonly(read_only);
    std::fs::set_permissions(path, permissions)
}

/// `GetFAttr(f, attr)`: the file's attributes. `ReadOnly` and `Directory`
/// come from the file itself, `Hidden` from a name starting with `.`, and
/// `Archive` (set for every file, as DOS did for new ones), `SysFile` and
/// a `Hidden` set by `SetFAttr` are remembered while the program runs.
/// As in DOS, writing to a file sets `Archive` again.
pub fn get_f_attr(f: &mut impl crate::io::AnyFile, attr: &mut u16) {
    let Some(path) = file_path(f) else { return };
    match std::fs::metadata(&path) {
        Ok(meta) => {
            let mut bits = if meta.is_dir() { DIRECTORY } else { ARCHIVE };
            if meta.permissions().readonly() {
                bits |= READ_ONLY;
            }
            if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                bits |= HIDDEN;
            }
            let extra = EXTRA_ATTRS.with_borrow(|attrs| {
                let extra = attrs.get(&file_id(&path, &meta))?;
                if extra.created != meta.created().ok() {
                    return None;
                }
                let written = extra.modified != meta.modified().ok();
                Some(if written { extra.bits | ARCHIVE } else { extra.bits })
            });
            if let Some(extra) = extra {
                bits = (bits & (READ_ONLY | DIRECTORY)) | extra;
            }
            *attr = u16::from(bits);
            set_dos_error(0);
        }
        Err(e) => set_dos_error(io_dos_error(&e, &path)),
    }
}

/// `SetFAttr(f, attr)`: `ReadOnly` really changes the file's permissions;
/// `Hidden`, `SysFile` and `Archive` are remembered (see `get_f_attr`).
pub fn set_f_attr(f: &mut impl crate::io::AnyFile, attr: impl Into<i32>) {
    let Some(path) = file_path(f) else { return };
    let attr = attr.into() as u8;
    let result = std::fs::metadata(&path).and_then(|meta| {
        set_read_only(&path, &meta, attr & READ_ONLY != 0)?;
        Ok(meta)
    });
    match result {
        Ok(meta) => {
            let extra = ExtraAttrs {
                bits: attr & (HIDDEN | SYS_FILE | ARCHIVE),
                created: meta.created().ok(),
                modified: meta.modified().ok(),
            };
            EXTRA_ATTRS.with_borrow_mut(|attrs| attrs.insert(file_id(&path, &meta), extra));
            set_dos_error(0);
        }
        Err(e) => set_dos_error(io_dos_error(&e, &path)),
    }
}

// ----- Interrupt vectors, faked -------------------------------------------

thread_local! {
    /// The 256 interrupt vectors. Each starts as a stand-in for the handler
    /// DOS had there, so a program that saves a vector and puts it back
    /// later works. (An `Rc` isn't thread-safe, hence `thread_local!`.)
    static VECTORS: std::cell::RefCell<Vec<AnyPtr>> = std::cell::RefCell::new(
        (0..256).map(|n| AnyPtr::code(&format!("DOS's handler for interrupt ${n:02X}"))).collect(),
    );
}

/// `GetIntVec(IntNo, Vector)`: what the vector holds. PascRust has no
/// interrupts, so a handler a program installs with `SetIntVec` is kept
/// but never called: a timer tick or key press handler just never runs.
pub fn get_int_vec(int_no: impl Into<i32>, vector: &mut AnyPtr) {
    let n = (int_no.into() & 0xFF) as usize;
    *vector = VECTORS.with_borrow(|vectors| vectors[n].clone());
}

/// `SetIntVec(IntNo, Vector)`: install a handler (which is never called).
pub fn set_int_vec(int_no: impl Into<i32>, vector: AnyPtr) {
    let n = (int_no.into() & 0xFF) as usize;
    VECTORS.with_borrow_mut(|vectors| vectors[n] = vector);
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
        assert!(dos_match("DATA?.DAT", "data.dat"));
        assert!(!dos_match("*.PAS", "."));
    }

    /// A scratch directory under `target/`, emptied.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/dos-tests").join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn searches_are_closed() {
        let dir = scratch("searches");
        std::fs::write(dir.join("one.txt"), "1").unwrap();
        let pattern = format!("{}/*.txt", dir.display());
        std::fs::write(dir.join("two.txt"), "2").unwrap();
        // A search still in use survives many that stop early, each in a
        // new SearchRec, and only so many stay open.
        let mut outer = SearchRec::default();
        find_first(pattern.as_str(), ARCHIVE, &mut outer);
        let (handle, _) = search_state(&outer);
        for _ in 0..OPEN_SEARCHES + 50 {
            find_first(pattern.as_str(), ARCHIVE, &mut SearchRec::default());
            // Walking the outer search uses it. (Back to its start each
            // time, so it never runs out.)
            set_search_state(&mut outer, handle, 0);
            find_next(&mut outer);
        }
        assert_eq!(SEARCHES.with_borrow(|s| s.len()), OPEN_SEARCHES);
        assert!(SEARCHES.with_borrow(|searches| searches.contains_key(&handle)));
    }

    #[test]
    fn a_walk_keeps_the_idle_parent_search() {
        let dir = scratch("walk");
        std::fs::write(dir.join("one.txt"), "1").unwrap();
        let pattern = format!("{}/*.txt", dir.display());
        // The parent's search waits, unused, while each subdirectory's
        // search is started and walked to its end.
        let mut parent = SearchRec::default();
        find_first(pattern.as_str(), ARCHIVE, &mut parent);
        let (handle, _) = search_state(&parent);
        for _ in 0..OPEN_SEARCHES + 50 {
            let mut child = SearchRec::default();
            find_first(pattern.as_str(), ARCHIVE, &mut child);
            find_next(&mut child);
        }
        assert!(SEARCHES.with_borrow(|searches| searches.contains_key(&handle)));
        // And only so many of the children, which have run out, are kept.
        // (The limit applies when a search starts; the last child ran out
        // after that.)
        let run_out = SEARCHES.with_borrow(|searches| searches.values().filter(|s| s.run_out).count());
        assert_eq!(run_out, RUN_OUT_SEARCHES + 1);
    }

    #[cfg(unix)]
    #[test]
    fn set_f_attr_gives_write_access_back_to_the_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let path = scratch("modes").join("mode.txt");
        std::fs::write(&path, "x").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let mut f = crate::io::PText::default();
        f.assign(path.to_string_lossy().as_ref());
        let mode = || std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        set_f_attr(&mut f, ARCHIVE);
        assert_eq!(mode(), 0o644);
        set_f_attr(&mut f, READ_ONLY);
        assert_eq!(mode(), 0o444);
        set_f_attr(&mut f, 0);
        assert_eq!(mode(), 0o644);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_directory_is_a_directory() {
        let dir = scratch("links");
        std::fs::create_dir(dir.join("sub")).unwrap();
        std::os::unix::fs::symlink(dir.join("sub"), dir.join("link")).unwrap();
        let mut s = SearchRec::default();
        find_first(format!("{}/link", dir.display()).as_str(), DIRECTORY, &mut s);
        assert_eq!((dos_error(), s.attr), (0, DIRECTORY));
    }

    #[test]
    fn pack_and_unpack_time() {
        let mut dt = DateTime { year: 1997, month: 3, day: 11, hour: 7, min: 30, sec: 41 };
        let mut packed = 0;
        pack_time(&mut dt, &mut packed);
        let mut back = DateTime::default();
        unpack_time(packed, &mut back);
        // Seconds are kept to 2, as DOS did.
        assert_eq!((back.year, back.month, back.day, back.hour, back.min, back.sec), (1997, 3, 11, 7, 30, 40));
        assert_eq!(packed as u32 >> 25, 17);
    }
}
