//! For a Rust program's `build.rs`: translate a Turbo Pascal unit while
//! Cargo builds the program, so its `cargo build` runs PascRust by itself.
//!
//! ```ignore
//! // build.rs
//! fn main() {
//!     pascrust::build::library("pascal/bstunit.pas").unwrap();
//! }
//!
//! // src/main.rs
//! pascrust_runtime::include_library!(bstunit);
//! ```
//!
//! [`library`] writes the unit (with the units it uses and the Rust files
//! it links) as one file, `bstunit.rs`, into Cargo's `OUT_DIR`, and tells
//! Cargo which files to watch, so it translates again only when one of
//! them changes. See docs/embedding.md.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::driver::{self, Options};

/// A unit that wouldn't translate. Its `Debug` is the error message as
/// PascRust renders it (file, line, the source and a caret), so
/// `.unwrap()` in a build script shows it as it should look.
pub struct Error(String);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Translate the unit at `path` (relative to the program's Cargo.toml) for
/// `pascrust_runtime::include_library!`.
pub fn library(path: impl AsRef<Path>) -> Result<(), Error> {
    library_with(path, &Options::default())
}

/// [`library`] with [`Options`]: more directories to find units in, and
/// compiler switches.
pub fn library_with(path: impl AsRef<Path>, options: &Options) -> Result<(), Error> {
    let path = path.as_ref();
    let generated = driver::transpile_library(path, options).map_err(Error)?;
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or_else(|| {
        Error("OUT_DIR isn't set: call pascrust::build::library from a Cargo build script (build.rs)".to_string())
    })?);
    let file = out_dir.join(format!("{}.rs", generated.name));
    std::fs::write(&file, single_file(&generated.files))
        .map_err(|e| Error(format!("cannot write {}: {e}", file.display())))?;
    // Translate again when any file it read changes (Cargo watches
    // `build.rs` itself anyway).
    for source in &generated.sources {
        let source = source.canonicalize().unwrap_or_else(|_| source.clone());
        println!("cargo:rerun-if-changed={}", source.display());
    }
    // A warning spans lines (it shows the source with a caret).
    for warning in &generated.warnings {
        for line in warning.lines() {
            println!("cargo:warning={line}");
        }
    }
    // A linked Rust file's `//! cargo:` crates can't be added for it.
    if !generated.dependencies.is_empty() {
        println!(
            "cargo:warning={}: its Rust files use crates; add them to [dependencies] in Cargo.toml: {}",
            path.display(),
            generated.dependencies.join(", ")
        );
    }
    Ok(())
}

/// The library's files as one, for `include!`: `lib.rs` with each module
/// inline (`mod helper { ... }`), since an included file can't have `mod
/// helper;` look for `helper.rs`, nor start with `//!` or `#![...]` (the
/// macro's module carries the `#[allow]`s instead).
fn single_file(files: &[(String, String)]) -> String {
    let (_, lib) = &files[0];
    let mut out = String::new();
    // The comment just above `#![allow(...)]` explains it; both go (the
    // macro's module has the `#[allow]`s).
    let lines: Vec<&str> = lib.lines().collect();
    let mut skip = vec![false; lines.len()];
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("#![") {
            skip[i] = true;
            let mut j = i;
            while j > 0 && lines[j - 1].trim().starts_with("//") && !lines[j - 1].trim().starts_with("//!") {
                j -= 1;
                skip[j] = true;
            }
        }
    }
    for (line, _) in lines.iter().zip(&skip).filter(|(_, skip)| !**skip) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("//!") {
            out.push_str(&format!("//{rest}\n"));
            continue;
        }
        // `mod helper;` or `pub mod helper;`: the module's file, inline.
        let declared = trimmed
            .strip_prefix("pub mod ")
            .map(|m| (true, m))
            .or_else(|| trimmed.strip_prefix("mod ").map(|m| (false, m)))
            .and_then(|(public, m)| m.strip_suffix(';').map(|m| (public, m)));
        match declared
            .and_then(|(public, m)| files.iter().find(|(f, _)| f == &format!("{m}.rs")).map(|f| (public, m, f)))
        {
            Some((public, module, (_, code))) => {
                out.push_str(&format!("{}mod {module} {{\n", if public { "pub " } else { "" }));
                out.push_str(code);
                if !code.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("}\n");
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_go_inline_and_inner_attributes_go() {
        let files = vec![
            (
                "lib.rs".to_string(),
                "//! Generated.\n// Why the allow is there.\n#![allow(dead_code)]\nmod counter;\npub mod helper;\npub use counter::*;\n"
                    .to_string(),
            ),
            ("counter.rs".to_string(), "//! Counter.\npub fn bump() {}\n".to_string()),
            ("helper.rs".to_string(), "pub fn tick() {}".to_string()),
        ];
        assert_eq!(
            single_file(&files),
            "// Generated.\nmod counter {\n//! Counter.\npub fn bump() {}\n}\npub mod helper {\npub fn tick() {}\n}\npub use counter::*;\n"
        );
    }
}
