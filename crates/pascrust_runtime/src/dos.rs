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

/// The `DosError` variable: the result of the last `Exec` (0 = success,
/// 2 = file not found).
pub fn dos_error() -> i16 {
    DOS_ERROR.with(Cell::get)
}

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
