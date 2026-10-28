//! The `pascrust` command-line tool.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

// `clap`'s derive API builds the argument parser from these type
// definitions: each field becomes an argument, and the doc comments become
// the `--help` text.
use clap::{Parser, Subcommand};

/// Pascrust: transpile Turbo Pascal programs to Rust.
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
            print!("{}", generated.main_rs);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Build { file, out } => {
            let dir = project(&file, out)?;
            cargo(&dir, &["build", "--quiet"], &[])
        }
        Cmd::Run { file, out, args } => {
            let dir = project(&file, out)?;
            cargo(&dir, &["run", "--quiet", "--"], &args)
        }
    }
}

/// Transpile `file` and write the Cargo project; return its directory.
fn project(file: &Path, out: Option<PathBuf>) -> Result<PathBuf, String> {
    let generated = pascrust::driver::transpile_file(file)?;
    let dir = out.unwrap_or_else(|| Path::new("out").join(&generated.name));
    pascrust::driver::write_project(&dir, &generated)
        .map_err(|e| format!("cannot write {}: {e}", dir.display()))?;
    Ok(dir)
}

fn cargo(dir: &Path, cargo_args: &[&str], program_args: &[String]) -> Result<ExitCode, String> {
    let status = Command::new("cargo")
        .args(cargo_args)
        .args(program_args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    // Pass the program's exit code through, so `Halt(3)` exits with 3.
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
