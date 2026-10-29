//! Turning Pascal identifiers into idiomatic Rust identifiers.
//!
//! Pascal programmers wrote `MaxCol`, `maxcol` or `MAXCOL` for the same name
//! (Pascal ignores case). Rust has naming conventions that the compiler
//! even warns about: `snake_case` for functions and variables, `CamelCase`
//! for types, `SCREAMING_SNAKE_CASE` for constants. We convert using the
//! spelling from the declaration.

/// Split an identifier into lowercase words: `MaxCol` → `["max", "col"]`,
/// `HTTPServer` → `["http", "server"]`, `grid_t` → `["grid", "t"]`.
fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        // Start a new word at a lowercase→uppercase boundary (`maxCol`), or
        // before the last capital of an acronym (`HTTPServer` → `HTTP|Server`).
        let prev = if i > 0 { chars[i - 1] } else { '_' };
        let next = chars.get(i + 1).copied().unwrap_or('_');
        let boundary = c.is_ascii_uppercase()
            && !current.is_empty()
            && (prev.is_ascii_lowercase()
                || prev.is_ascii_digit()
                || (prev.is_ascii_uppercase() && next.is_ascii_lowercase()));
        if boundary {
            words.push(std::mem::take(&mut current));
        }
        current.push(c.to_ascii_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

pub fn snake_case(name: &str) -> String {
    escape(words(name).join("_"))
}

pub fn screaming_snake_case(name: &str) -> String {
    escape(words(name).join("_").to_ascii_uppercase())
}

pub fn camel_case(name: &str) -> String {
    let joined: String = words(name)
        .iter()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    escape(joined)
}

/// Rust keywords, plus names our generated code and runtime prelude use.
/// A Pascal identifier that collides gets a trailing underscore.
const RESERVED: &[&str] = &[
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "gen",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "try",
    "union",
    "main",
    "pascal_main",
    "result",
    "w",
    "wd",
    "pstr",
    "halt",
    "rte",
    "run",
    // The runtime's types and traits, which every program imports.
    "PString",
    "Ordinal",
    "PArray",
    "PSet",
    "Ptr",
    "PText",
    "PFile",
    "FileRecord",
    "PasWrite",
    "Enumerated",
    // Runtime functions the generated code calls that no Pascal name spells
    // (`Random` is `random_real()`, `for c := Red to Blue` uses
    // `enum_range`), so a program's own `RandomReal` mustn't take their place.
    "enum_range",
    "random_real",
    "random_word",
    "io_unchecked",
    "pas_str",
    // The setters of runtime variables (`RandSeed := 1` is `set_rand_seed(1)`).
    "set_exit_code",
    "set_rand_seed",
    "set_file_mode",
    "set_text_attr",
    "set_last_mode",
    "set_check_break",
    "set_check_eof",
    "set_check_snow",
    "set_direct_video",
    "set_dos_error",
    "set_ovr_file_mode",
    "set_ovr_load_count",
    "set_ovr_result",
    "set_ovr_trap_count",
];

fn escape(name: String) -> String {
    if name.is_empty() {
        "_".to_string()
    } else if RESERVED.contains(&name.as_str()) || name.starts_with(|c: char| c.is_ascii_digit()) {
        name + "_"
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit variable's setter has no Pascal spelling, so a program's own
    /// `SetTextAttr` must be renamed rather than take its place.
    #[test]
    fn runtime_setters_are_reserved() {
        for unit in ["crt", "dos", "graph", "overlay"] {
            for var in crate::sema::units::unit_vars(unit) {
                assert!(RESERVED.contains(&var.set), "{} is not reserved", var.set);
            }
        }
    }

    #[test]
    fn conversions() {
        assert_eq!(snake_case("MaxCol"), "max_col");
        assert_eq!(snake_case("maxcol"), "maxcol");
        assert_eq!(snake_case("GridT"), "grid_t");
        assert_eq!(snake_case("HTTPServer"), "http_server");
        assert_eq!(snake_case("Type"), "type_");
        assert_eq!(snake_case("Exit"), "exit");
        assert_eq!(screaming_snake_case("maxRows"), "MAX_ROWS");
        assert_eq!(camel_case("gridT"), "GridT");
        assert_eq!(camel_case("node_ptr"), "NodePtr");
        assert_eq!(camel_case("BSTUnit"), "BstUnit");
    }
}
