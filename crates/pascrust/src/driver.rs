//! The driver ties the stages together and turns their output into a Cargo
//! project that `cargo build` can compile.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::ast::Unit;
use crate::diag::{Diagnostic, SourceMap, Span};
use crate::parser::SourceFile;
use crate::{codegen, parser, sema};

/// A transpiled program, ready to be written out as a Cargo project.
#[derive(Debug, Clone)]
pub struct Generated {
    /// The Cargo package name (from the file name, e.g. `hello`).
    pub name: String,
    /// The files of `src/`: `main.rs` first, then one module per unit.
    pub files: Vec<(String, String)>,
    /// Runtime features to enable, one per Turbo Pascal unit used (`crt`, ...).
    pub features: Vec<String>,
    /// Warnings to show the user, already rendered.
    pub warnings: Vec<String>,
}

impl Generated {
    pub fn main_rs(&self) -> &str {
        &self.files[0].1
    }

    /// All files in one string, each with a header line, for snapshots and
    /// `pascrust emit`.
    pub fn combined(&self) -> String {
        if self.files.len() == 1 {
            return self.files[0].1.clone();
        }
        let parts: Vec<String> =
            self.files.iter().map(|(name, code)| format!("// ===== src/{name} =====\n{code}")).collect();
        parts.join("\n")
    }
}

/// The units Turbo Pascal provides itself; everything else in a `uses`
/// clause must be a `.pas` file next to the program.
const STANDARD_UNITS: &[&str] = &["system", "crt", "dos", "graph", "printer"];

/// Transpile the Pascal program at `path`, together with every unit it uses.
/// Errors come back already rendered for printing, with file names, line
/// numbers and carets.
pub fn transpile_file(path: &Path) -> Result<Generated, String> {
    transpile_file_with(path, &Options::default())
}

/// Settings that come from the command line rather than the source, as with
/// Turbo Pascal's `TPC` (see `src/bin/tpc.rs`).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Options {
    /// More directories to look for units in, after the program's own.
    pub unit_dirs: Vec<PathBuf>,
    /// Compiler switches that apply from the start of every file, as if
    /// each began with `{$I-}` (for `I-`), and so on.
    pub switches: Vec<String>,
}

/// [`transpile_file`] with command-line [`Options`].
pub fn transpile_file_with(path: &Path, options: &Options) -> Result<Generated, String> {
    let source = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file_name = path.file_name().map_or_else(|| "input.pas".into(), |n| n.to_string_lossy().into_owned());
    let mut dirs = vec![path.parent().unwrap_or(Path::new(".")).to_path_buf()];
    dirs.extend(options.unit_dirs.iter().cloned());
    transpile(source, &file_name, &dirs, &options.switches)
}

/// Transpile a program held in memory. `file_name` is used in messages and
/// to name the package. Units can't be found without a directory to look in.
pub fn transpile_source(source: &[u8], file_name: &str) -> Result<Generated, String> {
    transpile(source.to_vec(), file_name, &[], &[])
}

fn transpile(source: Vec<u8>, file_name: &str, dirs: &[PathBuf], switches: &[String]) -> Result<Generated, String> {
    let mut map = SourceMap::new();
    let base = map.add(file_name, source.clone());
    let (program, directives) = parser::parse_file(&source, base).map_err(|d| map.render(&d))?;
    let program = match program {
        SourceFile::Program(program) => program,
        SourceFile::Unit(unit) => {
            return Err(format!("{file_name} is a unit; give pascrust the program that uses it ({})", unit.name.name));
        }
    };

    let mut loader = UnitLoader { dirs, map, loaded: Vec::new(), visiting: Vec::new(), directives };
    for name in &program.uses {
        loader.load(name)?;
    }
    let UnitLoader { map, loaded, directives, .. } = loader;
    // Command-line switches act like directives at the start of each file.
    // (Their warnings, if any, are given once, at the start of the program.)
    let at_start = |file: usize| {
        let base = map.file(file).base;
        switches.iter().map(move |text| parser::Directive { text: text.clone(), span: Span::new(base, base) })
    };
    let warnings: Vec<parser::Directive> = at_start(0).chain(directives.iter().cloned()).collect();
    let warnings = check_directives(&warnings).map_err(|d| map.render(&d))?;
    let mut with_switches: Vec<parser::Directive> = Vec::new();
    for file in 0..map.len() {
        with_switches.extend(at_start(file));
        with_switches.extend(directives.iter().filter(|d| map.file_index(d.span.start) == file).cloned());
    }
    let io_unchecked = io_unchecked_regions(&with_switches, &map);
    let warnings = warnings.iter().map(|d| map.render(d)).collect();
    let units: Vec<Unit> = loaded.into_iter().map(|(_, unit)| unit).collect();

    let analysis = sema::analyze(&program, &units)
        .map_err(|errors| errors.iter().map(|d| map.render(d)).collect::<Vec<_>>().join("\n\n"))?;
    let files = codegen::generate(&program, &units, &analysis, &map, &io_unchecked);
    let files = files.into_iter().map(|(name, code)| (name, format_rust(&code))).collect();

    let mut features: Vec<String> = program
        .uses
        .iter()
        .chain(units.iter().flat_map(|u| u.uses()))
        .map(|u| u.key())
        .filter(|u| matches!(u.as_str(), "crt" | "dos" | "graph"))
        .collect();
    features.sort();
    features.dedup();
    Ok(Generated { name: package_name(file_name), files, features, warnings })
}

/// Compiler directives (`{$R+}`, `{$I-}`, ...) are mostly ignored, but the
/// user should hear about the ones that change what a program does.
/// An include file (`{$I name}`) is an error: without it the program
/// would be missing code.
fn check_directives(directives: &[parser::Directive]) -> Result<Vec<Diagnostic>, Diagnostic> {
    let mut warnings = Vec::new();
    for d in directives {
        let text = d.text.trim();
        // `{$I file.inc}` includes a file; `{$I-}` and `{$I+}` are switches.
        if let Some(name) = text.strip_prefix(['I', 'i']).filter(|rest| rest.starts_with(' ')) {
            return Err(Diagnostic::new(d.span, format!("include files aren't supported (`{{$I {}}}`)", name.trim())));
        }
        // Switches can be combined: `{$R+,B-}`.
        for switch in text.split(',') {
            let switch = switch.trim().to_ascii_uppercase();
            let meaning = match switch.as_str() {
                "B+" => "complete Boolean evaluation; `and` and `or` still short-circuit",
                "R+" => "range checks on subranges; only array indexes are checked",
                "Q+" => "overflow checks; integer arithmetic still wraps",
                _ => continue,
            };
            warnings.push(Diagnostic::warning(d.span, format!("`{{${switch}}}` isn't supported: no {meaning}")));
        }
    }
    Ok(warnings)
}

/// The parts of the source compiled with `{$I-}`: from each `{$I-}` to the
/// next `{$I+}` or the end of that file. `{$I-}` is a compile-time switch, so
/// it applies to the text it covers, not to whatever runs in between.
fn io_unchecked_regions(directives: &[parser::Directive], map: &SourceMap) -> Vec<Span> {
    let mut regions = Vec::new();
    // Where the current `{$I-}` region started, and in which file.
    let mut open: Option<(usize, usize)> = None;
    let close = |regions: &mut Vec<Span>, (start, file): (usize, usize), end: Option<usize>| {
        let source = map.file(file);
        regions.push(Span::new(start, end.unwrap_or(source.base + source.text.len())));
    };
    for d in directives {
        let file = map.file_index(d.span.start);
        if let Some(region) = open.filter(|&(_, f)| f != file) {
            close(&mut regions, region, None);
            open = None;
        }
        for switch in d.text.split(',') {
            match switch.trim().to_ascii_uppercase().as_str() {
                "I-" if open.is_none() => open = Some((d.span.end, file)),
                "I+" => {
                    if let Some(region) = open.take() {
                        close(&mut regions, region, Some(d.span.start));
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(region) = open {
        close(&mut regions, region, None);
    }
    regions
}

/// Finds, parses and orders the units a program uses.
struct UnitLoader<'a> {
    /// Where to look for units, in order.
    dirs: &'a [PathBuf],
    map: SourceMap,
    /// Loaded units in dependency order: each after the units it uses.
    loaded: Vec<(String, Unit)>,
    /// Units being loaded right now, to detect `uses` cycles.
    visiting: Vec<String>,
    /// The compiler directives of every file loaded so far.
    directives: Vec<parser::Directive>,
}

impl UnitLoader<'_> {
    fn load(&mut self, name: &crate::ast::Ident) -> Result<(), String> {
        let key = name.key();
        if STANDARD_UNITS.contains(&key.as_str()) || self.loaded.iter().any(|(k, _)| *k == key) {
            return Ok(());
        }
        if self.visiting.contains(&key) {
            let at =
                crate::diag::Diagnostic::new(name.span, format!("units `{}` use each other in a cycle", name.name));
            return Err(self.map.render(&at));
        }
        let not_found = |map: &SourceMap| {
            map.render(&crate::diag::Diagnostic::new(
                name.span,
                format!("cannot find unit `{}` (looked for {key}.pas)", name.name),
            ))
        };
        let Some(path) = self.dirs.iter().find_map(|dir| find_file(dir, &format!("{key}.pas"))) else {
            return Err(not_found(&self.map));
        };
        let source = std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let base = self.map.add(file_name, source.clone());
        let (unit, directives) = parser::parse_file(&source, base).map_err(|d| self.map.render(&d))?;
        self.directives.extend(directives);
        let unit = match unit {
            SourceFile::Unit(unit) => unit,
            SourceFile::Program(_) => return Err(format!("{} is a program, not a unit", path.display())),
        };
        // Depth-first: load what this unit uses before adding the unit
        // itself, which yields dependency order.
        self.visiting.push(key.clone());
        for used in unit.uses().cloned().collect::<Vec<_>>() {
            self.load(&used)?;
        }
        self.visiting.pop();
        self.loaded.push((key, unit));
        Ok(())
    }
}

/// Find `name` in `dir`, ignoring case: DOS file names weren't
/// case-sensitive, so `uses BSTUnit` finds `bstunit.pas`.
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.file_name().is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(name)))
}

/// A valid Cargo package name from a file name: `Magic Square.pas` →
/// `magic_square`.
fn package_name(file_name: &str) -> String {
    let stem = file_name.rsplit_once('.').map_or(file_name, |(stem, _)| stem);
    let name: String =
        stem.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) || name.is_empty() { format!("p_{name}") } else { name }
}

/// Run `rustfmt` over generated code. If rustfmt isn't installed or fails
/// (which would mean codegen produced invalid Rust), return the code as is so
/// that `rustc` can report the problem.
pub fn format_rust(code: &str) -> String {
    let child = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return code.to_string();
    };
    // Write stdin and then drop it (the `take`), so rustfmt sees end of input.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(code.as_bytes());
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).into_owned(),
        _ => code.to_string(),
    }
}

/// Where the `pascrust_runtime` crate lives. `env!` reads an environment
/// variable *at compile time*; Cargo sets `CARGO_MANIFEST_DIR` to this
/// crate's directory, and the runtime is its sibling.
pub fn runtime_path() -> PathBuf {
    match std::env::var_os("PASCRUST_RUNTIME") {
        Some(path) => PathBuf::from(path),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../pascrust_runtime"),
    }
}

/// The `Cargo.toml` of a generated project.
pub fn cargo_toml(name: &str, features: &[String]) -> String {
    let runtime = runtime_path();
    let runtime = runtime.canonicalize().unwrap_or(runtime);
    let features = features.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(", ");
    format!(
        r#"# Generated by pascrust.
[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

# An empty workspace table keeps this project independent of any Cargo
# workspace it happens to sit inside.
[workspace]

[dependencies]
pascrust_runtime = {{ path = "{}", features = [{features}] }}

# Turbo Pascal integer arithmetic silently wraps around on overflow; Rust's
# would panic in debug builds. Turning off overflow checks gives TP behaviour.
[profile.dev]
overflow-checks = false

[profile.release]
overflow-checks = false
"#,
        runtime.display()
    )
}

/// Build the Cargo project in `dir` (written by [`write_project`] for
/// `generated`) and return the path of the executable.
pub fn build(dir: &Path, generated: &Generated) -> Result<PathBuf, String> {
    let status = Command::new("cargo")
        .args(["build", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build failed for {}", dir.display()));
    }
    let exe = dir.join("target").join("debug").join(format!("{}{}", generated.name, std::env::consts::EXE_SUFFIX));
    Ok(exe)
}

/// Write `generated` as a Cargo project in `dir`.
pub fn write_project(dir: &Path, generated: &Generated) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(dir.join("Cargo.toml"), cargo_toml(&generated.name, &generated.features))?;
    for (name, code) in &generated.files {
        std::fs::write(dir.join("src").join(name), code)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives_that_change_meaning_give_warnings() {
        let source = b"program P;\n{$R+,B-}\n{$I-}\n{$M 16384,0,0}\n{$Q+}\nbegin\nend.\n";
        let generated = transpile_source(source, "p.pas").unwrap();
        assert_eq!(generated.warnings.len(), 2);
        assert!(generated.warnings[0].starts_with("p.pas:2:1: warning: `{$R+}`"));
        assert!(generated.warnings[1].starts_with("p.pas:5:1: warning: `{$Q+}`"));
    }

    #[test]
    fn io_checks_switch_off_between_directives() {
        let mut map = SourceMap::new();
        map.add("a.pas", vec![b' '; 100]);
        map.add("b.pas", vec![b' '; 50]);
        let d = |text: &str, start| parser::Directive { text: text.to_string(), span: Span::new(start, start + 5) };
        // Off from 10 to 40 and from 60 to the end of a.pas. b.pas starts on
        // with checks on, then turns them off (with another switch) to its end.
        let b = map.file(1).base;
        let directives = [d("I-", 10), d("I+", 40), d("I-", 60), d("R+,I-", b + 20)];
        let regions = io_unchecked_regions(&directives, &map);
        assert_eq!(regions, vec![Span::new(15, 40), Span::new(65, 100), Span::new(b + 25, b + 50)]);
    }
}
