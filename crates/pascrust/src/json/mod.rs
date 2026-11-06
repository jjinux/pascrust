//! `pascrust to-json` and `pascrust from-json`: a binary file written by a
//! Turbo Pascal program (`file of T`) as JSON, and back.
//!
//! A typed file is just its records' bytes, one after another, in TP's
//! layout; nothing in it says what the records look like. So the Pascal
//! source that declares the record type comes too, and sema's view of that
//! type (the same one codegen uses for a translated program's files, see
//! `tp_size`) says where each field's bytes are. See docs/data_files.md.

mod decode;

use std::path::Path;

use crate::driver::{self, Options};
use crate::sema::types::tp_size;
use crate::sema::{Analysis, SymKind, Type};

/// The records of the file `data` as JSON (an array, pretty-printed), read
/// as the type `type_name` declared in the program or unit `source` (or,
/// without a name, as the element type of its only `file of`).
pub fn to_json(source: &Path, data: &Path, type_name: Option<&str>, options: &Options) -> Result<String, String> {
    let analysis = driver::analyze_file(source, options)?;
    let bytes = std::fs::read(data).map_err(|e| format!("cannot read {}: {e}", data.display()))?;
    let ty = record_type(&analysis, source, type_name, Some(bytes.len()))?;
    let size = tp_size(&ty).filter(|&size| size > 0).ok_or_else(|| format!("{ty} can't be in a typed file"))?;
    let size = size as usize;
    if bytes.len() % size != 0 {
        return Err(format!(
            "{} is {} bytes, which isn't a whole number of {ty} records ({size} bytes each): {} bytes are left over. Is it the right type?",
            data.display(),
            bytes.len(),
            bytes.len() % size
        ));
    }
    let records = bytes
        .chunks(size)
        .enumerate()
        .map(|(i, record)| decode::value(&analysis, &ty, record, &format!("[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut json = serde_json::to_string_pretty(&serde_json::Value::Array(records)).map_err(|e| e.to_string())?;
    json.push('\n');
    Ok(json)
}

/// The type a file's records have: the one named `type_name`, or, without
/// a name, the `T` of the only `file of T` in the source (if there are
/// several, the only one whose size divides the file's length, `data_len`).
fn record_type(
    analysis: &Analysis,
    source: &Path,
    type_name: Option<&str>,
    data_len: Option<usize>,
) -> Result<Type, String> {
    if let Some(name) = type_name {
        let found: Vec<&Type> = analysis
            .symbols
            .iter()
            .filter(|s| s.name.eq_ignore_ascii_case(name))
            .filter_map(|s| match &s.kind {
                SymKind::TypeName(ty) => Some(ty),
                _ => None,
            })
            .collect();
        return match found.as_slice() {
            [] => Err(format!("{} declares no type named `{name}`", source.display())),
            // `--type MasterFileType` (a `file of People`): its records.
            [Type::File(elem)] => Ok((**elem).clone()),
            [ty] => Ok((*ty).clone()),
            [first, rest @ ..] if rest.iter().all(|ty| ty == first) => Ok((*first).clone()),
            _ => Err(format!("{} and the units it uses declare more than one type named `{name}`", source.display())),
        };
    }
    // Every `file of T` sema saw, each type once.
    let mut candidates: Vec<&Type> = Vec::new();
    for ty in &analysis.file_types {
        if !candidates.contains(&ty) {
            candidates.push(ty);
        }
    }
    if candidates.len() > 1
        && let Some(len) = data_len
    {
        let fits: Vec<&Type> = candidates
            .iter()
            .copied()
            .filter(|ty| tp_size(ty).is_some_and(|size| size > 0 && len % size as usize == 0))
            .collect();
        if fits.len() == 1 {
            candidates = fits;
        }
    }
    match candidates.as_slice() {
        [ty] => Ok((*ty).clone()),
        [] => Err(format!("{} has no `file of` type; say which type with --type", source.display())),
        many => {
            let names: Vec<String> = many.iter().map(|ty| format!("`{ty}`")).collect();
            Err(format!(
                "{} has more than one `file of` type ({}); say which with --type",
                source.display(),
                names.join(", ")
            ))
        }
    }
}
