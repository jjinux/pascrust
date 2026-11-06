//! PascRust: a Turbo Pascal to Rust transpiler.
//!
//! The pipeline, one module per stage:
//!
//! ```text
//! source bytes ─lexer─▶ tokens ─parser─▶ AST ─sema─▶ Analysis ─codegen─▶ Rust source ─driver─▶ Cargo project
//! ```

pub mod ast;
// For a Rust program's build.rs: translate Pascal while Cargo builds.
pub mod build;
pub mod codegen;
pub mod diag;
pub mod driver;
pub mod fmt;
// `pascrust to-json` and `from-json`: a `file of T` as JSON and back.
#[cfg(feature = "cli")]
pub mod json;
pub mod lexer;
pub mod parser;
pub mod sema;
pub mod token;
