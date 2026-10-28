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
