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
    /// Crates the linked Rust files use (`rand = "0.8"`), for Cargo.toml.
    pub dependencies: Vec<String>,
    /// Every file the translation read: the units (as found on disk) and
    /// the linked Rust files (and the program, from [`transpile_file`]),
    /// for a `build.rs` to watch.
    pub sources: Vec<PathBuf>,
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
const STANDARD_UNITS: &[&str] = &["system", "crt", "dos", "graph", "printer", "overlay", "strings", "windos"];

/// PascRust's own units, written in Turbo Pascal and translated with the
/// program that uses them: (name, file name, source). A file of the same
/// name next to the program wins. (`include_str!` puts the file's text into
/// the `pascrust` binary when it's compiled.)
pub const PASCRUST_UNITS: &[(&str, &str, &str)] = &[
    ("prjson", "PRJson.pas", include_str!("../units/PRJson.pas")),
    ("prturtle", "PRTurtle.pas", include_str!("../units/PRTurtle.pas")),
];

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
    // `game.pas` has the parent `""`, which means the current directory.
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut dirs = vec![parent.to_path_buf()];
    dirs.extend(options.unit_dirs.iter().cloned());
    let mut generated = transpile(source, &file_name, &dirs, &options.switches, None)?;
    generated.sources.insert(0, path.to_path_buf());
    Ok(generated)
}

/// Transpile the unit at `path`, with the units it uses, as a Rust library
/// a Rust program can call (`pascrust build --lib`): `lib.rs` declares each
/// unit's module and re-exports the unit's interface.
pub fn transpile_library(path: &Path, options: &Options) -> Result<Generated, String> {
    let (generated, _) = transpile_unit(path, options, true)?;
    let file_name = path.file_name().map_or_else(|| "unit.pas".into(), |n| n.to_string_lossy().into_owned());
    // Graph's window needs the main thread, which only a Pascal program's
    // `run` gives it; called from a Rust program, `InitGraph` would wait
    // forever.
    if generated.features.iter().any(|f| f == "graph") {
        return Err(format!(
            "{file_name} uses Graph (itself or through a unit it uses), so it can't be a library: Graph's window needs the main thread, which only a Pascal program has. Make it a Pascal program that calls Rust instead (`{{$L file.rs}}`, see docs/calling_rust.md)"
        ));
    }
    Ok(generated)
}

/// True if the file at `path` is a unit (`unit Name; interface ...`).
pub fn is_unit(path: &Path) -> bool {
    let Ok(source) = std::fs::read(path) else { return false };
    let mut map = SourceMap::new();
    let base = map.add("unit.pas", source.clone());
    matches!(parser::parse_file(&source, base), Ok((SourceFile::Unit(_), ..)))
}

/// Transpile the unit at `path` with the units it uses, as a program that
/// uses it would (checking it compiles): the result, and the unit's name.
/// With `library`, its `lib.rs` (in place of `main.rs`) makes it a library.
pub fn transpile_unit(path: &Path, options: &Options, library: bool) -> Result<(Generated, String), String> {
    let source = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file_name = path.file_name().map_or_else(|| "unit.pas".into(), |n| n.to_string_lossy().into_owned());
    let mut map = SourceMap::new();
    let base = map.add(file_name.clone(), source.clone());
    let unit = match parser::parse_file(&source, base).map_err(|d| map.render(&d))?.0 {
        SourceFile::Unit(unit) => unit,
        SourceFile::Program(_) => {
            return Err(format!("{file_name} is a program, not a unit"));
        }
    };
    // Units are found by name, as in TP, so the file must be named after
    // the unit.
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if stem != unit.name.key() {
        return Err(format!(
            "{file_name} holds the unit `{}`; it must be in a file named after it, {}.pas, as in Turbo Pascal (that's how `uses` finds it)",
            unit.name.name,
            unit.name.key()
        ));
    }
    // A program that only uses the unit loads it and the units it uses,
    // as `pascrust build` would; its `main.rs` is then replaced.
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut dirs = vec![parent.to_path_buf()];
    dirs.extend(options.unit_dirs.iter().cloned());
    let program = format!("program Library;\nuses {};\nbegin\nend.\n", unit.name.name);
    let library = library.then(|| unit.name.key());
    let generated = transpile(program.into_bytes(), &file_name, &dirs, &options.switches, library.as_deref())?;
    Ok((generated, unit.name.name.clone()))
}

/// Check the program or unit at `path` (with the units it uses) and
/// return what sema found, without generating code: `to-json` and
/// `from-json` need its types. A unit is checked as a program that uses it.
pub fn analyze_file(path: &Path, options: &Options) -> Result<sema::Analysis, String> {
    let source = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file_name = path.file_name().map_or_else(|| "input.pas".into(), |n| n.to_string_lossy().into_owned());
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut dirs = vec![parent.to_path_buf()];
    dirs.extend(options.unit_dirs.iter().cloned());
    let source = if is_unit(path) {
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        format!("program Library;\nuses {name};\nbegin\nend.\n").into_bytes()
    } else {
        source
    };
    let mut map = SourceMap::new();
    let base = map.add(file_name.as_str(), source.clone());
    let (program, directives, _) = parser::parse_file(&source, base).map_err(|d| map.render(&d))?;
    let SourceFile::Program(program) = program else {
        unreachable!("a unit was wrapped in a program above");
    };
    let mut loader = UnitLoader {
        dirs: &dirs,
        map,
        loaded: Vec::new(),
        visiting: Vec::new(),
        directives,
        comments: Vec::new(),
        sources: Vec::new(),
    };
    for name in &program.uses {
        loader.load(name)?;
    }
    let units: Vec<Unit> = loader.loaded.into_iter().map(|(_, unit)| unit).collect();
    let map = loader.map;
    // An include file (`{$I name}`) would leave declarations out.
    check_directives(&loader.directives).map_err(|d| map.render(&d))?;
    sema::analyze(&program, &units)
        .map_err(|errors| errors.iter().map(|d| map.render(d)).collect::<Vec<_>>().join("\n\n"))
}

/// Transpile a program held in memory. `file_name` is used in messages and
/// to name the package. Units can't be found without a directory to look in.
pub fn transpile_source(source: &[u8], file_name: &str) -> Result<Generated, String> {
    transpile(source.to_vec(), file_name, &[], &[], None)
}

/// `library`: the unit (lowercase) to make a library of, for `--lib`.
fn transpile(
    source: Vec<u8>,
    file_name: &str,
    dirs: &[PathBuf],
    switches: &[String],
    library: Option<&str>,
) -> Result<Generated, String> {
    let mut map = SourceMap::new();
    let base = map.add(file_name, source.clone());
    let (program, directives, comments) = parser::parse_file(&source, base).map_err(|d| map.render(&d))?;
    let program = match program {
        SourceFile::Program(program) => program,
        SourceFile::Unit(unit) => {
            return Err(format!(
                "{file_name} is a unit; give `pascrust` the program that uses it ({})",
                unit.name.name
            ));
        }
    };

    let mut loader =
        UnitLoader { dirs, map, loaded: Vec::new(), visiting: Vec::new(), directives, comments, sources: Vec::new() };
    for name in &program.uses {
        loader.load(name)?;
    }
    let UnitLoader { map, loaded, directives, comments, sources, .. } = loader;
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
    let overflow_checked = switch_regions(&with_switches, &map, "Q+", "Q-");
    let warnings = warnings.iter().map(|d| map.render(d)).collect();
    let units: Vec<Unit> = loaded.into_iter().map(|(_, unit)| unit).collect();

    let mut analysis = sema::analyze(&program, &units)
        .map_err(|errors| errors.iter().map(|d| map.render(d)).collect::<Vec<_>>().join("\n\n"))?;
    let mut linked = linked_rust(&directives, dirs).map_err(|d| map.render(&d))?;
    // A unit `Sound` linking `sound.rs`: the Rust file's module is `sound_rs`.
    for l in &mut linked {
        if analysis.units.iter().any(|u| u.module == l.module) {
            l.module.push_str("_rs");
        }
    }
    link_externals(&mut analysis, &linked, &map).map_err(|d| map.render(&d))?;
    let mut warnings: Vec<String> = warnings;
    warnings.extend(analysis.warnings.iter().map(|d| map.render(d)));
    let modules: Vec<(String, String)> = linked.iter().map(|l| (l.module.clone(), l.file_name.clone())).collect();
    let mut features: Vec<String> = program
        .uses
        .iter()
        .chain(units.iter().flat_map(|u| u.uses()))
        .map(|u| u.key())
        // `WinDos` is in the runtime's `Dos` code.
        .map(|u| if u == "windos" { "dos".to_string() } else { u })
        .filter(|u| matches!(u.as_str(), "crt" | "dos" | "graph"))
        .collect();
    features.sort();
    features.dedup();
    let library = library.and_then(|key| units.iter().find(|u| u.name.key() == key));
    let files = codegen::generate(&codegen::Input {
        program: &program,
        units: &units,
        analysis: &analysis,
        map: &map,
        io_unchecked: &io_unchecked,
        overflow_checked: &overflow_checked,
        comments: &comments,
        rust_modules: &modules,
        features: &features,
        library,
    });
    let mut files: Vec<(String, String)> = files.into_iter().map(|(name, code)| (name, format_rust(&code))).collect();
    // The linked Rust files go in as they are.
    files.extend(linked.iter().map(|l| (format!("{}.rs", l.module), l.source.clone())));
    let dependencies = linked.iter().flat_map(|l| l.dependencies.clone()).collect();

    let mut sources = sources;
    sources.extend(linked.iter().map(|l| l.path.clone()));
    Ok(Generated { name: package_name(file_name), files, features, warnings, dependencies, sources })
}

/// A Rust file a program links with `{$L sound.rs}`, as TP linked an
/// `.OBJ` file: its `external` routines are there.
struct LinkedRust {
    /// Where the `{$L ...}`s that name it are.
    spans: Vec<Span>,
    /// Its module in the generated crate: `sound`.
    module: String,
    /// Its file name, as the `{$L}` gives it: `sound.rs`.
    file_name: String,
    /// Where it is (two units may link the same file).
    path: PathBuf,
    source: String,
    /// Crates it uses, from its `//! cargo: rand = "0.8"` lines.
    dependencies: Vec<String>,
}

/// The `{$L file}` directives: each names a Rust file next to the program
/// (or in a unit directory). An `.OBJ` file is x86 code for DOS, which
/// PascRust can't link.
fn linked_rust(directives: &[parser::Directive], dirs: &[PathBuf]) -> Result<Vec<LinkedRust>, Diagnostic> {
    let mut linked: Vec<LinkedRust> = Vec::new();
    for d in directives {
        let text = d.text.trim();
        let Some(name) = text.strip_prefix(['L', 'l']).filter(|rest| rest.starts_with(' ')).map(str::trim) else {
            continue;
        };
        // `{$L rust\sound.rs}`: DOS paths use backslashes.
        let name = name.replace('\\', "/");
        let path = Path::new(&name);
        if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("rs")) {
            return Err(Diagnostic::new(
                d.span,
                format!(
                    "PascRust can't link `{name}`: an object file is code for DOS; write the routines in Rust and link the `.rs` file (`{{$L sound.rs}}`)"
                ),
            ));
        }
        let file_name = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        let found =
            dirs.iter().find_map(|dir| find_file(&dir.join(path.parent().unwrap_or(Path::new(""))), &file_name));
        let Some(found) = found else {
            return Err(Diagnostic::new(d.span, format!("cannot find the Rust file `{name}`")));
        };
        let source = std::fs::read_to_string(&found)
            .map_err(|e| Diagnostic::new(d.span, format!("cannot read {}: {e}", found.display())))?;
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let module = sema::names::snake_case(&stem);
        let found = found.canonicalize().unwrap_or(found);
        if let Some(earlier) = linked.iter_mut().find(|l| l.module == module) {
            if earlier.path == found {
                earlier.spans.push(d.span);
                continue;
            }
            return Err(Diagnostic::new(
                d.span,
                format!(
                    "`{name}` and `{}` would both be the Rust module `{module}`; give one another name",
                    earlier.file_name
                ),
            ));
        }
        // `//! cargo: rand = "0.8"` adds a dependency to Cargo.toml.
        let dependencies = source
            .lines()
            .filter_map(|line| line.trim().strip_prefix("//! cargo:"))
            .map(|dep| dep.trim().to_string())
            .collect();
        linked.push(LinkedRust { spans: vec![d.span], module, file_name, path: found, source, dependencies });
    }
    Ok(linked)
}

/// Point each `external` routine at its Rust function: in the module of
/// the last `{$L file.rs}` before it in its file, named in snake case
/// (`Beep` is `sound::beep`).
fn link_externals(analysis: &mut sema::Analysis, linked: &[LinkedRust], map: &SourceMap) -> Result<(), Diagnostic> {
    for &(pid, span) in &analysis.externals.clone() {
        let file = map.file_index(span.start);
        let module = linked
            .iter()
            .flat_map(|l| l.spans.iter().map(move |s| (s, l)))
            .filter(|(s, _)| map.file_index(s.start) == file && s.start < span.start)
            .max_by_key(|(s, _)| s.start)
            .map(|(_, l)| l.module.clone());
        let Some(module) = module else {
            let name = &analysis.procs[pid].name;
            return Err(Diagnostic::new(
                span,
                format!("`{name}` is `external`, but no `{{$L file.rs}}` before it says which Rust file has it"),
            ));
        };
        // (`snake_case` gives a Rust keyword a trailing underscore: `loop_`.)
        let function = sema::names::snake_case(&analysis.procs[pid].name);
        analysis.procs[pid].rust_name = format!("{module}::{function}");
    }
    Ok(())
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
    switch_regions(directives, map, "I-", "I+")
}

/// The parts of the source where a switch is in the state `on` (`"Q+"`):
/// from each `on` to the next `off` (`"Q-"`) or the end of that file.
fn switch_regions(directives: &[parser::Directive], map: &SourceMap, on: &str, off: &str) -> Vec<Span> {
    let mut regions = Vec::new();
    // Where the current region (from `on`) started, and in which file.
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
                s if s == on && open.is_none() => open = Some((d.span.end, file)),
                s if s == off => {
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
    /// The comments of every file loaded so far, for codegen.
    comments: Vec<crate::lexer::Comment>,
    /// The files of the units loaded from disk (not PascRust's own).
    sources: Vec<PathBuf>,
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
            // TP 4.0 to 7.0 came with `Graph3` and `Turbo3` so that Turbo
            // Pascal 3.0 programs still compiled; PascRust doesn't have them.
            let message = if key == "graph3" || key == "turbo3" {
                format!(
                    "`{}` is Turbo Pascal's unit for running Turbo Pascal 3.0 programs, which PascRust doesn't provide (looked for {key}.pas)",
                    name.name
                )
            } else {
                format!("cannot find unit `{}` (looked for {key}.pas)", name.name)
            };
            map.render(&crate::diag::Diagnostic::new(name.span, message))
        };
        let (file_name, source) = match self.dirs.iter().find_map(|dir| find_file(dir, &format!("{key}.pas"))) {
            Some(path) => {
                let source = std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
                self.sources.push(path.clone());
                (path.file_name().unwrap().to_string_lossy().into_owned(), source)
            }
            None => match PASCRUST_UNITS.iter().find(|(k, ..)| *k == key) {
                Some((_, file_name, text)) => (file_name.to_string(), text.as_bytes().to_vec()),
                None => return Err(not_found(&self.map)),
            },
        };
        let base = self.map.add(file_name.clone(), source.clone());
        let (unit, directives, comments) = parser::parse_file(&source, base).map_err(|d| self.map.render(&d))?;
        self.directives.extend(directives);
        self.comments.extend(comments);
        let unit = match unit {
            SourceFile::Unit(unit) => unit,
            SourceFile::Program(_) => return Err(format!("{file_name} is a program, not a unit")),
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
    cargo_toml_with(name, features, &[])
}

/// The `Cargo.toml` of a generated project whose linked Rust files use
/// `dependencies` (`rand = "0.8"`).
pub fn cargo_toml_with(name: &str, features: &[String], dependencies: &[String]) -> String {
    let runtime = runtime_path();
    let runtime = runtime.canonicalize().unwrap_or(runtime);
    let features = features.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(", ");
    format!(
        r#"# Generated by PascRust.
[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

# An empty workspace table keeps this project independent of any Cargo
# workspace it happens to sit inside.
[workspace]

[dependencies]
pascrust_runtime = {{ path = "{}", features = [{features}] }}
{}
# Turbo Pascal integer arithmetic silently wraps around on overflow; Rust's
# would panic in debug builds. Turning off overflow checks gives TP behaviour.
[profile.dev]
overflow-checks = false

[profile.release]
overflow-checks = false
"#,
        runtime.display(),
        dependencies.iter().map(|d| format!("{d}\n")).collect::<String>()
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
    std::fs::write(
        dir.join("Cargo.toml"),
        cargo_toml_with(&generated.name, &generated.features, &generated.dependencies),
    )?;
    for (name, code) in &generated.files {
        std::fs::write(dir.join("src").join(name), code)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A linked Rust file's `//! cargo:` lines become dependencies in the
    /// generated Cargo.toml.
    #[test]
    fn linked_rust_files_bring_their_crates() {
        let dir = std::env::temp_dir().join(format!("pascrust_link_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Dice.rs"),
            "//! cargo: rand = \"0.8\"\n//! cargo:  itoa = \"1\"\npub fn roll() -> i16 { 4 }\n",
        )
        .unwrap();
        let directive = parser::Directive { text: "L dice.rs".to_string(), span: Span::new(0, 0) };
        let linked = linked_rust(&[directive], std::slice::from_ref(&dir)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(linked[0].module, "dice");
        assert_eq!(linked[0].dependencies, ["rand = \"0.8\"", "itoa = \"1\""]);
        let toml = cargo_toml_with("game", &[], &linked[0].dependencies);
        assert!(toml.contains("features = [] }\nrand = \"0.8\"\nitoa = \"1\"\n"), "{toml}");
    }

    #[test]
    fn directives_that_change_meaning_give_warnings() {
        let source = b"program P;\n{$R+,B-}\n{$I-}\n{$M 16384,0,0}\n{$Q+}\nbegin\nend.\n";
        let generated = transpile_source(source, "p.pas").unwrap();
        // (`{$Q+}` is supported: overflow checks.)
        assert_eq!(generated.warnings.len(), 1);
        assert!(generated.warnings[0].starts_with("p.pas:2:1: warning: `{$R+}`"));
    }

    #[test]
    fn installing_an_interrupt_handler_gives_a_warning() {
        let source = b"program P;\nuses Dos;\nvar old: Pointer;\nprocedure Tick; interrupt;\nbegin\nend;\nbegin\n  GetIntVec($1C, old);\n  SetIntVec($1C, @Tick);\n  SetIntVec($1C, old);\nend.\n";
        let generated = transpile_source(source, "p.pas").unwrap();
        assert_eq!(generated.warnings.len(), 1, "{:?}", generated.warnings);
        assert!(generated.warnings[0].starts_with("p.pas:9:3: warning: PascRust has no interrupts"));
        assert!(generated.warnings[0].contains("`Tick` will never be called; for a steady 18.2 ticks a second"));
        // Other vectors get no advice about timers.
        let source =
            b"program P;\nuses Dos;\nprocedure Key; interrupt;\nbegin\nend;\nbegin\n  SetIntVec($09, @Key);\nend.\n";
        let generated = transpile_source(source, "p.pas").unwrap();
        assert!(!generated.warnings[0].contains("steady"), "{}", generated.warnings[0]);
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
