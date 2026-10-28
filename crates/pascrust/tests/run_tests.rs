//! Execution tests: every `tests/run/NAME.pas` is transpiled, compiled and
//! run, and its standard output is compared with `tests/run/NAME.out`.
//!
//! Optional files next to the program:
//! - `NAME.in`: fed to the program as standard input
//! - `NAME.exitcode`: the expected exit code (default 0)
//!
//! Compiling each program as its own Cargo project would rebuild the runtime
//! every time, so all programs become binaries (`src/bin/NAME.rs`) of one
//! scratch package and a single `cargo build` compiles them all.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

#[test]
fn run_tests() {
    let dir = repo_root().join("tests/run");
    let mut programs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "pas"))
        .collect();
    programs.sort();
    // `PASCRUST_TEST=name` runs just one test, which is handy while debugging.
    if let Ok(only) = std::env::var("PASCRUST_TEST") {
        programs.retain(|p| p.file_stem().unwrap().to_string_lossy() == only);
    }

    // `CARGO_TARGET_TMPDIR` is a scratch directory Cargo provides to
    // integration tests.
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("run-tests");
    let bin_dir = scratch.join("src/bin");
    let _ = std::fs::remove_dir_all(&bin_dir);
    std::fs::create_dir_all(&bin_dir).unwrap();

    let mut failures = Vec::new();
    let mut built = Vec::new();
    let mut features = vec!["crt".to_string(), "dos".to_string()];
    for path in &programs {
        match pascrust::driver::transpile_file(path) {
            Ok(generated) => {
                // Each program is a binary in its own directory, so its unit
                // modules can sit next to its `main.rs`.
                let dir = bin_dir.join(&generated.name);
                std::fs::create_dir_all(&dir).unwrap();
                for (name, code) in &generated.files {
                    std::fs::write(dir.join(name), code).unwrap();
                }
                features.extend(generated.features);
                built.push((path.clone(), generated.name));
            }
            // Units are compiled as part of the programs that use them.
            Err(message) if message.contains("is a unit") => {}
            Err(message) => failures.push(format!("{}: transpile failed:\n{message}", path.display())),
        }
    }
    features.sort();
    features.dedup();
    let mut toml = pascrust::driver::cargo_toml("pascrust_run_tests", &features);
    // `cargo_toml` writes a binary package; we only need its dependency and
    // profile sections, with autodiscovered `src/bin/*.rs` binaries.
    toml = toml.replace("[workspace]", "[workspace]\n\n[lib]\npath = \"src/lib.rs\"");
    std::fs::write(scratch.join("Cargo.toml"), toml).unwrap();
    std::fs::write(scratch.join("src/lib.rs"), "").unwrap();

    let build = Command::new("cargo").args(["build", "--quiet"]).current_dir(&scratch).output().unwrap();
    if !build.status.success() {
        panic!("generated code failed to compile:\n{}", String::from_utf8_lossy(&build.stderr));
    }

    for (path, name) in built {
        let exe = scratch.join("target/debug").join(&name);
        let input = std::fs::read(path.with_extension("in")).unwrap_or_default();
        // Run in an empty scratch directory, so files the program creates
        // don't land in the repository.
        let cwd = scratch.join("cwd").join(&name);
        let _ = std::fs::remove_dir_all(&cwd);
        std::fs::create_dir_all(&cwd).unwrap();
        let mut child = Command::new(&exe)
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PASCRUST_FAST", "1")
            .env("PASCRUST_GRAPH", "headless")
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&input).unwrap();
        let output = child.wait_with_output().unwrap();
        let expected = std::fs::read(path.with_extension("out")).unwrap_or_default();
        if output.stdout != expected {
            failures.push(format!(
                "{}: output differs\n--- expected\n{}\n--- actual\n{}\n--- stderr\n{}",
                path.display(),
                String::from_utf8_lossy(&expected),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        let expected_code: i32 =
            std::fs::read_to_string(path.with_extension("exitcode")).map(|s| s.trim().parse().unwrap()).unwrap_or(0);
        if output.status.code() != Some(expected_code) {
            failures.push(format!(
                "{}: exit code {:?}, expected {expected_code}\n{}",
                path.display(),
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    if !failures.is_empty() {
        panic!("{} run test failure(s):\n\n{}", failures.len(), failures.join("\n\n"));
    }
}
