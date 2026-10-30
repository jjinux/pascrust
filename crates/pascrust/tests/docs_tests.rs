//! Every error that links to `docs/rust_limits.md` links to a section that
//! exists there, so the advice it promises is really on the page.

use std::path::Path;

#[test]
fn rewrite_links_have_sections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("../../docs/rust_limits.md")).unwrap();
    let mut anchors = Vec::new();
    for dir in ["src/sema", "src/codegen", "src"] {
        for entry in std::fs::read_dir(root.join(dir)).unwrap().flatten() {
            let Ok(source) = std::fs::read_to_string(entry.path()) else { continue };
            // `rewrite_link("two-var-arguments")`, ignoring its definition.
            for piece in source.split("rewrite_link(\"").skip(1) {
                anchors.push(piece.split('"').next().unwrap().to_string());
            }
        }
    }
    assert!(!anchors.is_empty(), "no rewrite_link calls found");
    for anchor in anchors {
        assert!(page.contains(&format!("<a id=\"{anchor}\"></a>")), "docs/rust_limits.md has no section `{anchor}`");
    }
}
