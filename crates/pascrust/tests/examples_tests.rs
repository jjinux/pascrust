//! The acceptance test: every program in `examples/` transpiles, and the
//! generated Rust builds without warnings. (They're interactive, so they
//! aren't run here; `README.md` says how to run each one.)
//!
//! As in `run_tests.rs`, each program becomes a binary of one scratch
//! package, so a single `cargo build` compiles them all.

use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn examples_build_without_warnings() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").canonicalize().unwrap();
    // The `.pas` files in `examples/` and one level down; units are skipped
    // (they're compiled with the programs that use them).
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            for inner in std::fs::read_dir(&path).unwrap() {
                files.push(inner.unwrap().path());
            }
        } else {
            files.push(path);
        }
    }
    files.retain(|p| p.extension().is_some_and(|e| e == "pas"));
    files.sort();

    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("examples");
    let bin_dir = scratch.join("src/bin");
    let _ = std::fs::remove_dir_all(&bin_dir);
    std::fs::create_dir_all(&bin_dir).unwrap();
    let mut features = Vec::new();
    let mut programs = 0;
    for path in &files {
        match pascrust::driver::transpile_file(path) {
            Ok(generated) => {
                assert!(generated.warnings.is_empty(), "{}: {:?}", path.display(), generated.warnings);
                let dir = bin_dir.join(&generated.name);
                std::fs::create_dir_all(&dir).unwrap();
                for (name, code) in &generated.files {
                    std::fs::write(dir.join(name), code).unwrap();
                }
                features.extend(generated.features);
                programs += 1;
            }
            Err(message) if message.contains("is a unit") => {}
            Err(message) => panic!("{}: transpile failed:\n{message}", path.display()),
        }
    }
    assert!(programs >= 10, "found only {programs} example programs");
    features.sort();
    features.dedup();
    let toml = pascrust::driver::cargo_toml("pascrust_examples", &features)
        .replace("[workspace]", "[workspace]\n\n[lib]\npath = \"src/lib.rs\"");
    std::fs::write(scratch.join("Cargo.toml"), toml).unwrap();
    std::fs::write(scratch.join("src/lib.rs"), "").unwrap();

    // `-D warnings` turns any warning in the generated code into an error.
    let build = Command::new("cargo")
        .args(["build", "--quiet"])
        .env("RUSTFLAGS", "-D warnings")
        .current_dir(&scratch)
        .output()
        .unwrap();
    assert!(build.status.success(), "the examples' Rust failed to build:\n{}", String::from_utf8_lossy(&build.stderr));
}
