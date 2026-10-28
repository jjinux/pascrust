//! Parse every `.pas` file given on the command line and report errors.
//! Run with `cargo run -p pascrust --example parse_all -- examples/**/*.pas`.
fn main() {
    for path in std::env::args().skip(1) {
        let source = std::fs::read(&path).unwrap();
        match pascrust::parser::parse_file(&source, 0) {
            Ok(_) => println!("ok    {path}"),
            Err(d) => println!("FAIL  {}", d.render(&path, &source).lines().next().unwrap()),
        }
    }
}
