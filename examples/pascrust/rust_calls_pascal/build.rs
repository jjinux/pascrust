//! Translate the Pascal unit into Rust as part of `cargo build`.

fn main() {
    pascrust::build::library("../../database/bstunit.pas").unwrap();
}
