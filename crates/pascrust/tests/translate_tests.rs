//! Translation tests: these pin down *which Rust* each Pascal construct
//! becomes. Every `tests/translate/*.pas` is transpiled and the result is
//! compared with a saved snapshot in `tests/snapshots/`.
//!
//! When codegen changes on purpose, review and accept the new snapshots with
//! `cargo insta review` (install with `cargo install cargo-insta`), or run
//! with `INSTA_UPDATE=always` and read the diff in git.
//!
//! `tests/errors/*.pas` are programs pascrust must reject; their snapshots
//! hold the error messages.

#[test]
fn translations() {
    // `glob!` runs the closure once per matching file, naming each snapshot
    // after the file.
    insta::glob!("../../../tests/translate", "*.pas", |path| {
        match pascrust::driver::transpile_file(path) {
            Ok(generated) => insta::assert_snapshot!(generated.combined()),
            // Units are snapshotted as part of the programs that use them.
            Err(e) if e.contains("is a unit") => {}
            Err(e) => panic!("{e}"),
        }
    });
}

#[test]
fn errors() {
    insta::glob!("../../../tests/errors", "*.pas", |path| {
        let error = match pascrust::driver::transpile_file(path) {
            Ok(_) => panic!("{} should not transpile", path.display()),
            Err(e) => e,
        };
        insta::assert_snapshot!(error);
    });
}

/// Attaching comments once took time proportional to comments × statements
/// × file size: a 700-line example took 20 seconds. This program has 3,000
/// lines and 2,000 comments.
#[test]
fn many_comments_transpile_quickly() {
    let mut source = String::from("program Long;\nvar n: Integer;\nbegin\n  n := 0;\n");
    for i in 0..1000 {
        source.push_str(&format!("  {{ step {i} }}\n  n := n + {i}; {{ add {i} }}\n  WriteLn(n);\n"));
    }
    source.push_str("end.\n");
    let start = std::time::Instant::now();
    let generated = pascrust::driver::transpile_source(source.as_bytes(), "long.pas").unwrap();
    assert!(generated.combined().contains("// add 999"));
    assert!(start.elapsed().as_secs() < 20, "took {:?}", start.elapsed());
}
