//! A Rust program whose `build.rs` runs PascRust (`pascrust::build`):
//! `examples/pascrust/rust_calls_pascal`, which uses the user's
//! `examples/database/bstunit.pas`. Cargo builds it, the build script
//! translates the unit, and the program's output must match
//! `tests/embed/rust_calls_pascal.out`.

use std::path::Path;
use std::process::Command;

#[test]
fn a_build_script_translates_the_pascal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let example = root.join("examples/pascrust/rust_calls_pascal");
    // The same scratch target as `embed_tests`, so the runtime is built once.
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("embed/target");
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--manifest-path"])
        .arg(example.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        // The generated code must build without warnings in the user's crate.
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .unwrap();
    assert!(output.status.success(), "the example failed:\n{}", String::from_utf8_lossy(&output.stderr));
    let expected = std::fs::read_to_string(root.join("tests/embed/rust_calls_pascal.out")).unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    // Its Cargo.toml turns overflow checks off, so no warning about them.
    assert!(!String::from_utf8_lossy(&output.stderr).contains("overflow checks"));

    // The build script told Cargo to translate again when the unit changes.
    let build_dir = target.join("debug/build");
    let told = std::fs::read_dir(&build_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("rust_calls_pascal-"))
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("output")).ok())
        .any(|output| output.lines().any(|l| l.starts_with("cargo:rerun-if-changed=") && l.ends_with("bstunit.pas")));
    assert!(told, "no rerun-if-changed for bstunit.pas in {}", build_dir.display());
}

/// Everything a build script must watch: the unit and the units it uses.
#[test]
fn the_sources_include_used_units() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let generated =
        pascrust::driver::transpile_library(&root.join("tests/embed/counter.pas"), &Default::default()).unwrap();
    let names: Vec<String> =
        generated.sources.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_ascii_lowercase()).collect();
    assert!(names.contains(&"counter.pas".to_string()) && names.contains(&"helper.pas".to_string()), "{names:?}");
}

/// A unit that uses Crt checks, when it compiles, that the runtime has
/// the `crt` feature (a Rust program chooses the runtime's features).
#[test]
fn a_crt_unit_checks_for_the_crt_feature() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("build_rs_crt");
    std::fs::create_dir_all(&scratch).unwrap();
    std::fs::write(
        scratch.join("screen.pas"),
        "unit Screen;\ninterface\nuses Crt;\nprocedure Blank;\nimplementation\nprocedure Blank;\nbegin\n  ClrScr;\nend;\nend.\n",
    )
    .unwrap();
    let generated = pascrust::driver::transpile_library(&scratch.join("screen.pas"), &Default::default()).unwrap();
    assert!(generated.files[0].1.contains("pascrust_runtime::FEATURE_CRT"), "{}", generated.files[0].1);
}

/// `tests/embed/counter.pas` through a build script: a unit with variables,
/// a unit it uses, and a linked Rust file that names its types `super::`.
/// The program is `counter_host.rs`, with `include_library!` in front.
#[test]
fn a_build_script_with_used_units_and_linked_rust() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    // (Its `//!` header can't follow the macro, so it's plain comments here.)
    let host = std::fs::read_to_string(root.join("tests/embed/counter_host.rs")).unwrap().replace("//!", "//");
    let output = cargo_run(
        "build_rs_counter",
        &root.join("tests/embed/counter.pas"),
        &format!("pascrust_runtime::include_library!(counter);\n\n{host}"),
        "overflow-checks = false",
    );
    assert!(output.status.success(), "the program failed:\n{}", String::from_utf8_lossy(&output.stderr));
    let expected = std::fs::read_to_string(root.join("tests/embed/counter_host.out")).unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

/// With `panic = "abort"`, a runtime error can't come back from `call` (nor
/// an I/O error under `{$I-}` be kept for `IOResult`): the program ends,
/// and stderr must say why instead of nothing.
#[test]
fn panic_abort_says_why_the_program_ends() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("build_rs_abort");
    std::fs::create_dir_all(&scratch).unwrap();
    let unit = scratch.join("opener.pas");
    std::fs::write(
        &unit,
        "unit Opener;\ninterface\nfunction Open: Integer;\nimplementation\n\
         function Open: Integer;\nvar\n   F: Text;\nbegin\n   Assign(F, 'no such file');\n   \
         {$I-}\n   Reset(F);\n   {$I+}\n   Open := IOResult;\nend;\nend.\n",
    )
    .unwrap();
    let main = "pascrust_runtime::include_library!(opener);\n\n\
                fn main() {\n    let c = pascrust_runtime::call(opener::OpenerContext::new).unwrap();\n    \
                println!(\"{:?}\", pascrust_runtime::call(|| c.open()));\n}\n";
    let output = cargo_run("build_rs_abort", &unit, main, "panic = \"abort\"");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("Runtime error 2:") && stderr.contains("panic = \"abort\""), "{stderr}");
}

/// `Halt` inside `call` with `panic = "abort"`: nothing could catch it as
/// an `Err`, so it ends the program with its exit code, as in Pascal.
#[test]
fn panic_abort_halt_exits_with_its_code() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("build_rs_abort_halt");
    std::fs::create_dir_all(&scratch).unwrap();
    let unit = scratch.join("stopper.pas");
    std::fs::write(
        &unit,
        "unit Stopper;\ninterface\nprocedure Stop;\nimplementation\nprocedure Stop;\nbegin\n   Halt(3);\nend;\nend.\n",
    )
    .unwrap();
    let main = "pascrust_runtime::include_library!(stopper);\n\n\
                fn main() {\n    let c = pascrust_runtime::call(stopper::StopperContext::new).unwrap();\n    \
                let _ = pascrust_runtime::call(|| c.stop());\n    println!(\"not reached\");\n}\n";
    let output = cargo_run("build_rs_abort_halt", &unit, main, "panic = \"abort\"");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(!stderr.contains("Halt") && output.stdout.is_empty(), "{stderr}");
}

/// Write a Rust program into `CARGO_TARGET_TMPDIR/name` whose build script
/// translates `unit`, with `main_rs` and `[profile.dev]` lines, and run it.
fn cargo_run(name: &str, unit: &Path, main_rs: &str, profile: &str) -> std::process::Output {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().unwrap();
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(scratch.join("src")).unwrap();
    std::fs::write(
        scratch.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n\n\
             [dependencies]\npascrust_runtime = {{ path = \"{}\" }}\n\n\
             [build-dependencies]\npascrust = {{ path = \"{}\", default-features = false }}\n\n\
             [profile.dev]\n{profile}\n",
            crates.join("pascrust_runtime").display(),
            crates.join("pascrust").display()
        ),
    )
    .unwrap();
    std::fs::write(
        scratch.join("build.rs"),
        format!("fn main() {{\n    pascrust::build::library({unit:?}).unwrap();\n}}\n"),
    )
    .unwrap();
    std::fs::write(scratch.join("src/main.rs"), main_rs).unwrap();
    Command::new("cargo")
        .args(["run", "--quiet", "--manifest-path"])
        .arg(scratch.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", Path::new(env!("CARGO_TARGET_TMPDIR")).join("embed/target"))
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .unwrap()
}
