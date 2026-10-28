//! `tpc`: a command-line compiler that takes Turbo Pascal's `TPC.EXE`
//! options, so old build scripts and habits carry over:
//!
//! ```text
//! tpc [options] filename [options]
//! ```
//!
//! `tpc game` (or `tpc game.pas`) writes the executable `game` next to the
//! source, as `TPC GAME` wrote `GAME.EXE`. Behind the scenes it is
//! `pascrust build`: the Cargo project lives in a cache directory, so all
//! that appears beside the source is the program.
//!
//! The options that still mean something are supported; the rest, which
//! were about DOS memory, map files and the IDE's debugger, are accepted
//! and ignored, with a note. Options start with `-` only: TP also took `/`,
//! but on Unix a `/` starts a path.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pascrust::driver::{self, Options};

const USAGE: &str = "\
Pascrust tpc: compile Turbo Pascal programs, with TPC's command line.

Syntax: tpc [options] filename [options]
  -$<switches>  Compiler switches, as in {$...}: '-$I-' or '-$R+,I-'
                (quoted, or the shell reads $I as a variable)
  -B            Build all units (always done: units are part of the program)
  -E<dir>       Where to write the executable (default: next to the source)
  -M            Make modified units (always done)
  -Q            Quiet: no messages unless something goes wrong
  -U<dirs>      Unit directories, separated by ';'

Accepted and ignored: -D (conditional defines), -F (find error), -G (map
file), -I and -O (include and object directories), -L (link buffer),
-T (TURBO.TPL directory), -V (debug info).";

/// What the command line asks for.
#[derive(Debug, Default, PartialEq)]
struct Args {
    file: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
    quiet: bool,
    options: Options,
    /// Notes about options that don't apply.
    notes: Vec<String>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args::default();
    for arg in args {
        let Some(option) = arg.strip_prefix('-') else {
            if parsed.file.is_some() {
                return Err(format!("only one file can be compiled at a time (found `{arg}` too)"));
            }
            parsed.file = Some(PathBuf::from(arg));
            continue;
        };
        // TP's options are one letter, in either case, then any value.
        let mut chars = option.chars();
        let letter = chars.next().map(|c| c.to_ascii_uppercase());
        let value = chars.as_str();
        match letter {
            Some('$') => parsed.options.switches.extend(value.split(',').map(|s| s.trim().to_string())),
            Some('B' | 'M') => {}
            Some('E') => parsed.exe_dir = Some(PathBuf::from(value)),
            Some('Q') => parsed.quiet = true,
            Some('U') => {
                parsed.options.unit_dirs.extend(value.split(';').filter(|d| !d.is_empty()).map(PathBuf::from));
            }
            Some('D') => parsed.notes.push(format!("ignoring {arg}: conditional compilation isn't supported")),
            Some('F' | 'G' | 'I' | 'L' | 'O' | 'T' | 'V') => {
                parsed.notes.push(format!("ignoring {arg}: it has no meaning for pascrust"));
            }
            _ => return Err(format!("invalid option `{arg}`\n\n{USAGE}")),
        }
    }
    Ok(parsed)
}

/// The source file: with `.pas` added if there's no extension, as TP added
/// `.PAS` (so `tpc game` never picks up the executable `game` from last
/// time). Names match files on disk ignoring case, as in DOS.
fn source_path(file: &Path) -> Option<PathBuf> {
    if file.extension().is_some() && file.is_file() {
        return Some(file.to_path_buf());
    }
    let dir = file.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = file.file_name()?.to_string_lossy().into_owned();
    let wanted = if file.extension().is_some() { name } else { format!("{name}.pas") };
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.file_name().is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(&wanted)))
}

/// Where the Cargo project for `name` is built: a cache directory, so
/// rebuilding is quick and nothing but the program appears next to the
/// source.
fn build_dir(name: &str) -> PathBuf {
    let cache = match std::env::var_os("XDG_CACHE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(".cache"),
            None => std::env::temp_dir(),
        },
    };
    cache.join("pascrust").join("tpc").join(name)
}

fn compile(args: &Args) -> Result<(), String> {
    let file = args.file.as_ref().ok_or(USAGE)?;
    let source = source_path(file).ok_or_else(|| format!("{}: file not found", file.display()))?;
    let generated = driver::transpile_file_with(&source, &args.options)?;
    for warning in &generated.warnings {
        eprintln!("{warning}\n");
    }
    let dir = build_dir(&generated.name);
    driver::write_project(&dir, &generated).map_err(|e| format!("cannot write {}: {e}", dir.display()))?;
    let built = driver::build(&dir, &generated)?;
    let exe_dir = match &args.exe_dir {
        Some(dir) => dir.clone(),
        None => source.parent().unwrap_or(Path::new(".")).to_path_buf(),
    };
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let exe = exe_dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    // Remove the old executable rather than copying over it: macOS caches
    // a program's code signature by file, so an executable rewritten in
    // place can be killed as soon as it starts.
    let _ = std::fs::remove_file(&exe);
    std::fs::copy(&built, &exe).map_err(|e| format!("cannot write {}: {e}", exe.display()))?;
    if !args.quiet {
        let lines = std::fs::read(&source).map_or(0, |text| text.iter().filter(|&&b| b == b'\n').count());
        println!("{}: {lines} lines, compiled to {}", source.display(), exe.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = parse_args(&args).and_then(|args| {
        if !args.quiet {
            for note in &args.notes {
                eprintln!("tpc: {note}");
            }
        }
        compile(&args)
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        parse_args(&list.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn options_before_and_after_the_file() {
        let parsed = args(&["-Q", "game", "-$R+,I-", "-Eout", "-uunits;lib", "-GD", "-B"]).unwrap();
        assert_eq!(parsed.file, Some(PathBuf::from("game")));
        assert!(parsed.quiet);
        assert_eq!(parsed.exe_dir, Some(PathBuf::from("out")));
        assert_eq!(parsed.options.switches, ["R+", "I-"]);
        assert_eq!(parsed.options.unit_dirs, [PathBuf::from("units"), PathBuf::from("lib")]);
        assert_eq!(parsed.notes, ["ignoring -GD: it has no meaning for pascrust"]);
    }

    #[test]
    fn bad_options() {
        assert!(args(&["-Z", "game"]).unwrap_err().starts_with("invalid option `-Z`"));
        assert!(args(&["a.pas", "b.pas"]).is_err());
    }
}
