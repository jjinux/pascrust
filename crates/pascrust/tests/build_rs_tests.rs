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
