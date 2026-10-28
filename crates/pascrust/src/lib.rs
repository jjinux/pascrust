//! Pascrust: a Turbo Pascal to Rust transpiler.
//!
//! The pipeline, one module per stage:
//!
//! ```text
//! source bytes ─lexer─▶ tokens ─parser─▶ AST ─sema─▶ Analysis ─codegen─▶ Rust source ─driver─▶ Cargo project
//! ```

pub mod ast;
pub mod codegen;
pub mod diag;
pub mod driver;
pub mod fmt;
pub mod lexer;
pub mod parser;
pub mod sema;
pub mod token;
