//! The `tpc` command: compile a program with TPC-style options and run the
//! executable it writes.

use std::process::Command;

#[test]
fn tpc_compiles_next_to_the_source_with_switches() {
    // `CARGO_TARGET_TMPDIR` is a scratch directory Cargo keeps between
    // runs, so the generated project's build is cached there.
    let tmp = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("tpc");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(
        tmp.join("Probe.pas"),
        "program Probe;\nvar f: Text; code: Integer;\nbegin\n  Assign(f, 'missing.txt');\n  Reset(f);\n  code := IOResult;\n  WriteLn('IOResult = ', code);\nend.\n",
    )
    .unwrap();
    // `-$I-` on the command line works like `{$I-}` at the top of the file.
    let status = Command::new(env!("CARGO_BIN_EXE_tpc"))
        .args(["-Q", "-$I-", "PROBE"])
        .current_dir(&tmp)
        .env("XDG_CACHE_HOME", tmp.join("cache"))
        .status()
        .unwrap();
    assert!(status.success());
    let output =
        Command::new(tmp.join(format!("Probe{}", std::env::consts::EXE_SUFFIX))).current_dir(&tmp).output().unwrap();
    assert!(output.status.success(), "{:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "IOResult = 2\n");
    // `-E` puts the executable in another directory, made if it's missing.
    let _ = std::fs::remove_dir_all(tmp.join("exe"));
    let status = Command::new(env!("CARGO_BIN_EXE_tpc"))
        .args(["-Q", "-Eexe/sub", "PROBE"])
        .current_dir(&tmp)
        .env("XDG_CACHE_HOME", tmp.join("cache"))
        .status()
        .unwrap();
    assert!(status.success());
    assert!(tmp.join(format!("exe/sub/Probe{}", std::env::consts::EXE_SUFFIX)).is_file());
}

/// TPC compiled a unit to a .TPU file; `tpc` checks one compiles and says
/// how Ruscal uses units.
#[test]
fn tpc_checks_a_unit() {
    let tmp = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("tpc_unit");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(
        tmp.join("Tally.pas"),
        "unit Tally;\ninterface\nfunction Twice(N: Integer): Integer;\nimplementation\nfunction Twice(N: Integer): Integer;\nbegin\n  Twice := N * 2;\nend;\nend.\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tpc")).arg("TALLY").current_dir(&tmp).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("the unit Tally compiles") && text.contains("ruscal build --lib"), "{text}");
}
