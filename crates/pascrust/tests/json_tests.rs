//! `pascrust to-json` (and `from-json`): binary files written by Turbo
//! Pascal programs, as JSON.
//!
//! The fixtures: `examples/database/mark.dat` and `mark.key`, written by
//! the user's database program in 1997, and `tests/json/items.dat` and
//! `raws.dat`, written by `tests/json/records.pas` (run it in `tests/json`
//! with `pascrust run records.pas` to make them again).

use std::path::{Path, PathBuf};

use pascrust::driver::Options;
use pascrust::json::{from_json, to_json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(source: &str, data: &str, type_name: Option<&str>) -> Result<String, String> {
    to_json(&root().join(source), &root().join(data), type_name, &Options::default())
}

/// `from-json` of `text` (written to a scratch file named `name`).
fn bytes(source: &str, name: &str, text: &str, type_name: Option<&str>) -> Result<Vec<u8>, String> {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("json");
    std::fs::create_dir_all(&scratch).unwrap();
    let file = scratch.join(name);
    std::fs::write(&file, text).unwrap();
    from_json(&root().join(source), &file, type_name, &Options::default())
}

#[test]
fn the_1997_database_files() {
    // `founit.pas` has two `file of` types; each file's size says which.
    let people = json("examples/database/founit.pas", "examples/database/mark.dat", None).unwrap();
    assert_eq!(
        people,
        "[\n  {\n    \"name\": \"Shannon Behrens\",\n    \"ID\": 1,\n    \"Index\": 2930802.0\n  }\n]\n"
    );
    let keys = json("examples/database/founit.pas", "examples/database/mark.key", None).unwrap();
    assert_eq!(
        keys,
        "[\n  {\n    \"ID\": 1,\n    \"Location\": 0,\n    \"Before\": null,\n    \"After\": null\n  }\n]\n"
    );
    // A type name is found whatever its case (`founit.pas` says `people`).
    let named = json("examples/database/founit.pas", "examples/database/mark.dat", Some("People")).unwrap();
    assert_eq!(named, people);
}

#[test]
fn every_kind_of_field() {
    let items = json("tests/json/records.pas", "tests/json/items.dat", Some("Item")).unwrap();
    insta::assert_snapshot!(items);
    // Without a tag, every variant, read from the same bytes.
    let raws = json("tests/json/records.pas", "tests/json/raws.dat", Some("Raw")).unwrap();
    assert_eq!(raws, "[\n  {\n    \"W\": 258,\n    \"Lo\": 2,\n    \"Hi\": 1\n  }\n]\n");
    // `--type` may name the file type itself.
    assert_eq!(json("tests/json/records.pas", "tests/json/raws.dat", Some("RawFile")).unwrap(), raws);
}

#[test]
fn errors() {
    // Both types' sizes divide 90 bytes (an Item is 45, a Raw 2).
    let ambiguous = json("tests/json/records.pas", "tests/json/items.dat", None).unwrap_err();
    assert!(ambiguous.contains("more than one `file of` type (`Item`, `Raw`)"), "{ambiguous}");
    let wrong_size = json("tests/json/records.pas", "tests/json/raws.dat", Some("Item")).unwrap_err();
    assert!(wrong_size.contains("2 bytes, which isn't a whole number of Item records (45 bytes each)"), "{wrong_size}");
    let unknown = json("tests/json/records.pas", "tests/json/raws.dat", Some("Nothing")).unwrap_err();
    assert!(unknown.contains("declares no type named `Nothing`"), "{unknown}");

    // A `Double` is 8 bytes in TP's files, but a `Real` (6) in PascRust's.
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("json_double");
    std::fs::create_dir_all(&scratch).unwrap();
    std::fs::write(
        scratch.join("prices.pas"),
        "program Prices;\ntype\n   Price = record\n      Cost: Double;\n   end;\nvar\n   F: file of Price;\nbegin\nend.\n",
    )
    .unwrap();
    std::fs::write(scratch.join("prices.dat"), [0; 6]).unwrap();
    let double =
        to_json(&scratch.join("prices.pas"), &scratch.join("prices.dat"), None, &Options::default()).unwrap_err();
    assert!(double.contains("[0].Cost is a Double"), "{double}");
}

/// `from-json` gives back the bytes `to-json` read: exactly, for files
/// whose strings were cleared before they were written.
#[test]
fn from_json_gives_the_bytes_back() {
    for (source, data, type_name) in [
        ("tests/json/records.pas", "tests/json/items.dat", "Item"),
        ("tests/json/records.pas", "tests/json/raws.dat", "Raw"),
        ("examples/database/founit.pas", "examples/database/mark.key", "PeopleKey"),
    ] {
        let text = json(source, data, Some(type_name)).unwrap();
        let back = bytes(source, &format!("round_trip_{type_name}.json"), &text, Some(type_name)).unwrap();
        assert_eq!(back, std::fs::read(root().join(data)).unwrap(), "{data}");
    }
    // In 1997 the bytes after the name were whatever was there before;
    // they come back as zeros, and the JSON is the same.
    let source = "examples/database/founit.pas";
    let text = json(source, "examples/database/mark.dat", None).unwrap();
    let back = bytes(source, "round_trip_mark.json", &text, Some("people")).unwrap();
    let original = std::fs::read(root().join("examples/database/mark.dat")).unwrap();
    assert_eq!(back.len(), original.len());
    assert_eq!(back[..16], original[..16]); // the length byte and the name
    assert!(back[16..31].iter().all(|&b| b == 0));
    assert_eq!(back[31..], original[31..]);
}

#[test]
fn from_json_errors() {
    let item =
        std::fs::read_to_string(root().join("crates/pascrust/tests/snapshots/json_tests__every_kind_of_field.snap"))
            .unwrap();
    // The first record of the snapshot, as JSON.
    let start = item.find("[\n").unwrap();
    let items: serde_json::Value = serde_json::from_str(&item[start..]).unwrap();
    let first = items[0].clone();
    let error = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut record = first.clone();
        edit(&mut record);
        let text = serde_json::Value::Array(vec![record]).to_string();
        bytes("tests/json/records.pas", "edited_item.json", &text, Some("Item")).unwrap_err()
    };
    // Each case: the message expected, and a change that causes it.
    type Edit = Box<dyn Fn(&mut serde_json::Value)>;
    let cases: Vec<(&str, Edit)> = vec![
        ("[0]: the field `Count` is missing", Box::new(|r| _ = r.as_object_mut().unwrap().remove("Count"))),
        ("[0]: `Colour` isn't a field here", Box::new(|r| r["Colour"] = "Red".into())),
        ("[0].Count is 300, which doesn't fit Byte (0..255)", Box::new(|r| r["Count"] = 300.into())),
        ("[0].Count should be a whole number, not 1.5", Box::new(|r| r["Count"] = 1.5.into())),
        ("[0].Name has 11 characters, but it's a string[10]", Box::new(|r| r["Name"] = "Eleven char".into())),
        ("isn't a character of code page 437", Box::new(|r| r["Letter"] = "€".into())),
        (
            "[0].Hue is \"Purple\", which isn't one of Color's values (Red, Green, Blue)",
            Box::new(|r| r["Hue"] = "Purple".into()),
        ),
        ("[0].Letters[0]: \"A\" can't be in a", Box::new(|r| r["Letters"] = serde_json::json!(["A"]))),
        ("[0].Grid[0] should have 3 elements, not 2", Box::new(|r| r["Grid"][0] = serde_json::json!([1, 2]))),
        ("[0].Next should be null", Box::new(|r| r["Next"] = 4.into())),
        // The tag says Circle, so Square's fields don't belong.
        ("[0]: `Side` isn't a field here", Box::new(|r| r["Side"] = 3.into())),
    ];
    for (expected, edit) in cases {
        let message = error(&*edit);
        assert!(message.contains(expected), "expected {expected:?} in {message:?}");
    }

    // Without a tag, fields of several variants are fine if they agree.
    let raw = |text: &str| bytes("tests/json/records.pas", "edited_raw.json", text, Some("Raw"));
    assert_eq!(raw(r#"[{"W": 258, "Lo": 2}]"#).unwrap(), [2, 1]);
    assert_eq!(raw(r#"[{"Hi": 1}]"#).unwrap(), [0, 1]);
    let clash = raw(r#"[{"W": 258, "Lo": 3}]"#).unwrap_err();
    assert!(clash.contains("[0].Lo and `W` share bytes"), "{clash}");

    let not_array = raw(r#"{"W": 1}"#).unwrap_err();
    assert!(not_array.contains("should hold an array"), "{not_array}");
}
