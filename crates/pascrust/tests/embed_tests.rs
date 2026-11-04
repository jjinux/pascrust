//! Rust calling Turbo Pascal: `pascrust build --lib` makes a library of a
//! unit, and a Rust program uses it. Here the unit is the user's
//! `examples/database/bstunit.pas`, and the program `tests/embed/bst_host.rs`;
//! its output must match `tests/embed/bst_host.out`.

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn a_rust_program_calls_a_pascal_unit() {
    run_host("examples/database/bstunit.pas", "bstunit", "bst_host");
}

/// A unit with variables: they're the fields of `CounterContext`.
#[test]
fn a_rust_program_owns_a_units_variables() {
    run_host("tests/embed/counter.pas", "counter", "counter_host");
}

/// The `lib.rs` that `--lib` writes for it, as a snapshot (review the diff
/// when it changes: `INSTA_UPDATE=always cargo test --test embed_tests`).
#[test]
fn the_libraries_lib_rs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let generated = pascrust::driver::transpile_library(
        &root.join("tests/embed/counter.pas"),
        &pascrust::driver::Options::default(),
    )
    .unwrap();
    insta::assert_snapshot!("counter_lib_rs", generated.files[0].1);
}

/// Build the unit at `unit` as the library `crate_name`, then the Rust
/// program `tests/embed/<host>.rs` that uses it, and check its output
/// against `tests/embed/<host>.out`.
fn run_host(unit: &str, crate_name: &str, host: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("embed");
    let generated =
        pascrust::driver::transpile_library(&root.join(unit), &pascrust::driver::Options::default()).unwrap();
    assert!(generated.warnings.is_empty(), "{:?}", generated.warnings);
    pascrust::driver::write_project(&scratch.join(crate_name), &generated).unwrap();

    // The Rust program, a package that depends on the library.
    let dir = scratch.join(host);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::copy(root.join(format!("tests/embed/{host}.rs")), dir.join("src/main.rs")).unwrap();
    let toml = format!(
        "[package]\nname = \"{host}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n\n\
         [dependencies]\n{crate_name} = {{ path = \"../{crate_name}\" }}\n"
    );
    std::fs::write(dir.join("Cargo.toml"), toml).unwrap();

    // The library must build without warnings, as generated programs do.
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .env("RUSTFLAGS", "-D warnings")
        // `Traverse` waits for Enter; with no input it goes straight on.
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{host} failed:\n{}", String::from_utf8_lossy(&output.stderr));
    let expected = std::fs::read_to_string(root.join(format!("tests/embed/{host}.out"))).unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

/// A unit that uses Graph can't be a library (its window needs the main
/// thread), and a unit must be in a file named after it.
#[test]
fn lib_refuses_graph_and_misnamed_files() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("embed_refused");
    std::fs::create_dir_all(&scratch).unwrap();
    let unit = |name: &str, uses: &str| {
        format!("unit {name};\ninterface\n{uses}procedure Go;\nimplementation\nprocedure Go;\nbegin\nend;\nend.\n")
    };
    std::fs::write(scratch.join("pictures.pas"), unit("Pictures", "uses Graph;\n")).unwrap();
    std::fs::write(scratch.join("misnamed.pas"), unit("Tally", "")).unwrap();
    let options = pascrust::driver::Options::default();
    let graph = pascrust::driver::transpile_library(&scratch.join("pictures.pas"), &options).unwrap_err();
    assert!(graph.contains("uses Graph") && graph.contains("calls Rust instead"), "{graph}");
    let misnamed = pascrust::driver::transpile_library(&scratch.join("misnamed.pas"), &options).unwrap_err();
    assert!(misnamed.contains("must be in a file named after it, tally.pas"), "{misnamed}");
}
