//! `pascrust fmt` on the examples: they stay formatted, and formatting never
//! changes a program's tokens.

use std::path::{Path, PathBuf};

use pascrust::fmt::format_files;
use pascrust::lexer::tokenize;
use pascrust::token::TokenKind;

fn examples() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files = Vec::new();
    let mut dirs = vec![root];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pas")) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// The tokens, with identifiers lowercased (formatting may re-case them).
fn tokens(source: &[u8]) -> Vec<TokenKind> {
    tokenize(source)
        .unwrap()
        .into_iter()
        .map(|t| match t.kind {
            TokenKind::Ident(name) => TokenKind::Ident(name.to_ascii_lowercase()),
            kind => kind,
        })
        .collect()
}

/// The examples are formatted together (identifiers are spelled the same
/// across all of them), so `pascrust fmt examples/**/*.pas` changes nothing.
#[test]
fn examples_are_formatted() {
    let files = examples();
    let sources: Vec<Vec<u8>> = files.iter().map(|f| std::fs::read(f).unwrap()).collect();
    let borrowed: Vec<&[u8]> = sources.iter().map(Vec::as_slice).collect();
    let formatted = format_files(&borrowed).unwrap();
    for ((file, old), new) in files.iter().zip(&sources).zip(&formatted) {
        assert!(old == new, "{} is not formatted; run `pascrust fmt` on all the examples", file.display());
    }
}

/// Strip all indentation and use DOS line endings, then format: the tokens
/// must come back unchanged.
#[test]
fn formatting_keeps_tokens() {
    let files = examples();
    let mangled: Vec<Vec<u8>> = files
        .iter()
        .map(|f| {
            let text = std::fs::read(f).unwrap();
            let lines: Vec<Vec<u8>> =
                text.split(|&c| c == b'\n').map(|l| l.iter().skip_while(|c| **c == b' ').copied().collect()).collect();
            lines.join(&b"\r\n"[..])
        })
        .collect();
    let borrowed: Vec<&[u8]> = mangled.iter().map(Vec::as_slice).collect();
    let formatted = format_files(&borrowed).unwrap();
    for ((file, before), after) in files.iter().zip(&mangled).zip(&formatted) {
        assert!(tokens(before) == tokens(after), "formatting changed the tokens of {}", file.display());
        assert!(!after.contains(&b'\r'), "{} still has DOS line endings", file.display());
    }
}
