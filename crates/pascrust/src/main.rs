//! The `pascrust` command-line tool.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

// `clap`'s derive API builds the argument parser from these type
// definitions: each field becomes an argument, and the doc comments become
// the `--help` text.
use clap::{Parser, Subcommand};

/// PascRust: transpile Turbo Pascal programs to Rust.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the generated Rust code.
    Emit { file: PathBuf },
    /// Transpile into a Cargo project and build it.
    Build {
        file: PathBuf,
        /// Where to write the Cargo project (default: out/<name>).
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Make a Rust library of a unit, for a Rust program to call.
        #[arg(long)]
        lib: bool,
    },
    /// Make indentation, spacing and identifier casing consistent. Give a
    /// program and its units together, so names are spelled the same in all.
    Fmt {
        files: Vec<PathBuf>,
        /// Don't write anything; list the files that would change, and fail
        /// if there are any.
        #[arg(long)]
        check: bool,
    },
    /// Transpile, build and run.
    Run {
        file: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Arguments for the Pascal program (after `--`).
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Print a binary file written by a Turbo Pascal program (`file of T`)
    /// as JSON, reading its records as a type the Pascal source declares.
    ToJson {
        /// The program or unit that declares the record type.
        source: PathBuf,
        /// The binary file.
        data: PathBuf,
        /// The record type (default: the only `file of` type in the source).
        #[arg(long = "type")]
        type_name: Option<String>,
        /// Where to write the JSON (default: standard output).
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// The reverse of `to-json`: write the records in a JSON file back as a
    /// binary file a Turbo Pascal program reads (`file of T`).
    FromJson {
        /// The program or unit that declares the record type.
        source: PathBuf,
        /// The JSON file (an array, one element per record).
        json: PathBuf,
        /// The record type (default: the only `file of` type in the source).
        #[arg(long = "type")]
        type_name: Option<String>,
        /// Where to write the binary file.
        #[arg(short, long)]
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // `?`-style error handling in `main` would print errors with `Debug`
    // formatting; a small inner function returning `Result` keeps the
    // messages readable.
    match run(cli.command) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Cmd) -> Result<ExitCode, String> {
    match command {
        Cmd::Emit { file } => {
            let generated = pascrust::driver::transpile_file(&file)?;
            warn(&generated);
            print!("{}", generated.combined());
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Build { file, out, lib } => {
            let dir = if lib { library(&file, out)? } else { project(&file, out)? };
            cargo(&dir, &["build", "--quiet"], &[])
        }
        Cmd::Fmt { files, check } => fmt(&files, check),
        Cmd::Run { file, out, args } => {
            let dir = project(&file, out)?;
            cargo(&dir, &["run", "--quiet", "--"], &args)
        }
        Cmd::ToJson { source, data, type_name, out } => {
            let options = pascrust::driver::Options::default();
            let json = pascrust::json::to_json(&source, &data, type_name.as_deref(), &options)?;
            match out {
                Some(out) => std::fs::write(&out, json).map_err(|e| format!("cannot write {}: {e}", out.display()))?,
                None => print!("{json}"),
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::FromJson { source, json, type_name, out } => {
            let options = pascrust::driver::Options::default();
            let bytes = pascrust::json::from_json(&source, &json, type_name.as_deref(), &options)?;
            std::fs::write(&out, bytes).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn fmt(files: &[PathBuf], check: bool) -> Result<ExitCode, String> {
    let sources = files
        .iter()
        .map(|f| std::fs::read(f).map_err(|e| format!("cannot read {}: {e}", f.display())))
        .collect::<Result<Vec<_>, _>>()?;
    let borrowed: Vec<&[u8]> = sources.iter().map(Vec::as_slice).collect();
    let formatted = pascrust::fmt::format_files(&borrowed)
        .map_err(|(i, d)| d.render(&files[i].display().to_string(), &sources[i]))?;
    let mut changed = false;
    for ((file, old), new) in files.iter().zip(&sources).zip(formatted) {
        if *old != new {
            changed = true;
            println!("{}", file.display());
            if !check {
                std::fs::write(file, new).map_err(|e| format!("cannot write {}: {e}", file.display()))?;
            }
        }
    }
    Ok(if check && changed { ExitCode::FAILURE } else { ExitCode::SUCCESS })
}

fn warn(generated: &pascrust::driver::Generated) {
    for warning in &generated.warnings {
        eprintln!("{warning}\n");
    }
}

/// Transpile `file` and write the Cargo project; return its directory.
fn project(file: &Path, out: Option<PathBuf>) -> Result<PathBuf, String> {
    let generated = pascrust::driver::transpile_file(file)?;
    warn(&generated);
    let dir = out.unwrap_or_else(|| Path::new("out").join(&generated.name));
    pascrust::driver::write_project(&dir, &generated).map_err(|e| format!("cannot write {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Transpile the unit `file` as a library and write the Cargo project.
fn library(file: &Path, out: Option<PathBuf>) -> Result<PathBuf, String> {
    let generated = pascrust::driver::transpile_library(file, &pascrust::driver::Options::default())?;
    warn(&generated);
    let dir = out.unwrap_or_else(|| Path::new("out").join(&generated.name));
    pascrust::driver::write_project(&dir, &generated).map_err(|e| format!("cannot write {}: {e}", dir.display()))?;
    Ok(dir)
}

fn cargo(dir: &Path, cargo_args: &[&str], program_args: &[String]) -> Result<ExitCode, String> {
    // `--manifest-path` rather than running cargo in `dir`, so the program
    // starts in the user's current directory: relative file names and drive
    // mappings like `PASCRUST_DRIVE_A=examples` mean what they'd expect.
    let (command, flags) = cargo_args.split_first().expect("a cargo command");
    let status = Command::new("cargo")
        .arg(command)
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .args(flags)
        .args(program_args)
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    // Pass the program's exit code through, so `Halt(3)` exits with 3.
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
