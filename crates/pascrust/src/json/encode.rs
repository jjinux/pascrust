//! JSON to bytes in Turbo Pascal's layout: the reverse of `decode`. It's
//! strict, since a wrong byte would corrupt the Pascal program's data: every
//! field must be there and nothing else, and every value must fit its type.

use pascrust_runtime::cp437;
use pascrust_runtime::io::real48;
use serde_json::{Map, Value};

use super::decode::has_virtual_methods;
use crate::sema::types::{Field, ObjectType, RealKind, RecordType, VariantPart, set_bytes, tp_size};
use crate::sema::{Analysis, IntKind, Type};

/// Where a value goes: its bytes, which of them mean something, and which
/// field wrote each. (A string's bytes after its length mean nothing, so two
/// variants of a record without a tag can share them without a clash; the
/// owners name the other field when they do clash.)
struct Out<'a> {
    bytes: &'a mut [u8],
    meant: &'a mut [bool],
    owners: &'a mut [Option<String>],
}

impl Out<'_> {
    /// Bytes `start..start + len` of this one.
    fn part(&mut self, start: usize, len: usize) -> Out<'_> {
        let range = start..start + len;
        Out {
            bytes: &mut self.bytes[range.clone()],
            meant: &mut self.meant[range.clone()],
            owners: &mut self.owners[range],
        }
    }

    fn set(&mut self, bytes: &[u8]) {
        self.bytes[..bytes.len()].copy_from_slice(bytes);
        self.meant[..bytes.len()].fill(true);
    }
}

/// The bytes of one record of type `ty`, from its JSON. `path` says where
/// it is (`[3].address.zip`), for messages.
pub fn record_bytes(analysis: &Analysis, ty: &Type, json: &Value, path: &str) -> Result<Vec<u8>, String> {
    let mut buffer = Buffer::new(size_of(ty)?);
    value(analysis, ty, json, &mut buffer.out(), path)?;
    Ok(buffer.bytes)
}

/// The storage an [`Out`] writes into.
struct Buffer {
    bytes: Vec<u8>,
    meant: Vec<bool>,
    owners: Vec<Option<String>>,
}

impl Buffer {
    fn new(size: usize) -> Self {
        Buffer { bytes: vec![0; size], meant: vec![false; size], owners: vec![None; size] }
    }

    fn out(&mut self) -> Out<'_> {
        Out { bytes: &mut self.bytes, meant: &mut self.meant, owners: &mut self.owners }
    }
}

fn size_of(ty: &Type) -> Result<usize, String> {
    tp_size(ty).map(|size| size as usize).ok_or_else(|| format!("{ty} can't be in a typed file"))
}

fn value(analysis: &Analysis, ty: &Type, json: &Value, out: &mut Out, path: &str) -> Result<(), String> {
    match ty {
        Type::Int(kind) => {
            let n = integer(*kind, json, path)?;
            let bytes = n.to_le_bytes();
            out.set(&bytes[..size_of(ty)?]);
        }
        Type::Bool => match json {
            Value::Bool(b) => out.set(&[u8::from(*b)]),
            _ => return Err(format!("{path} should be true or false, not {json}")),
        },
        Type::Char => {
            let s = string(json, path)?;
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => out.set(&[character(c, path)?]),
                _ => return Err(format!("{path} should be one character, not {json}")),
            }
        }
        Type::Str { cap } => {
            let s = string(json, path)?;
            let bytes = s.chars().map(|c| character(c, path)).collect::<Result<Vec<u8>, _>>()?;
            if bytes.len() > usize::from(*cap) {
                return Err(format!("{path} has {} characters, but it's a string[{cap}]", bytes.len()));
            }
            out.set(&[bytes.len() as u8]);
            out.part(1, bytes.len()).set(&bytes);
        }
        Type::Real(RealKind::Real) => {
            let x = json.as_f64().ok_or_else(|| format!("{path} should be a number, not {json}"))?;
            // A `Real` goes from about 2.9e-39 to 1.7e38 (and 0).
            if x.abs() > 1.7e38 {
                return Err(format!("{path} is {x}, too big for a Real"));
            }
            if x != 0.0 && x.abs() < 2.9e-39 {
                return Err(format!("{path} is {x}, too small for a Real (it would be 0)"));
            }
            out.set(&real48::from_f64(x));
        }
        Type::Real(kind) => {
            return Err(format!("{path} is a {kind:?}: PascRust writes only 6-byte `Real`s to files for now"));
        }
        Type::Enum(_) => {
            let ordinal = ordinal(ty, json, path)?;
            out.set(&ordinal.to_le_bytes()[..size_of(ty)?]);
        }
        Type::Set(elem, _) => {
            let Value::Array(elements) = json else {
                return Err(format!("{path} should be an array of the set's elements, not {json}"));
            };
            let (first, count) = set_bytes(ty);
            let mut bytes = vec![0_u8; count];
            for (i, element) in elements.iter().enumerate() {
                let ordinal = ordinal(elem, element, &format!("{path}[{i}]"))?;
                let byte = (ordinal / 8) as usize;
                if ordinal < 0 || byte < first || byte >= first + count {
                    return Err(format!("{path}[{i}]: {element} can't be in a {ty}"));
                }
                bytes[byte - first] |= 1 << (ordinal % 8);
            }
            out.set(&bytes);
        }
        Type::Array(array) => {
            let Value::Array(elements) = json else {
                return Err(format!("{path} should be an array, not {json}"));
            };
            let count = array.count() as usize;
            if elements.len() != count {
                return Err(format!("{path} should have {count} elements, not {}", elements.len()));
            }
            let size = size_of(&array.elem)?;
            for (i, element) in elements.iter().enumerate() {
                value(analysis, &array.elem, element, &mut out.part(i * size, size), &format!("{path}[{i}]"))?;
            }
        }
        Type::Record(record) => {
            let fields = object(json, path)?;
            let mut keys = Keys::new(fields, path);
            self::record(analysis, record, &mut keys, out, path)?;
            keys.done()?;
        }
        Type::Object(object) => {
            if has_virtual_methods(analysis, object) {
                return Err(format!("{path} is an object with virtual methods, which PascRust can't write to a file"));
            }
            let fields = self::object(json, path)?;
            let mut keys = Keys::new(fields, path);
            object_fields(analysis, object, &mut keys, out, path)?;
            keys.done()?;
        }
        // An address or a file handle can't be written back: nil (zeros).
        Type::Pointer(_) | Type::AnyPointer | Type::PChar | Type::Proc(_) | Type::Text | Type::File(_) => {
            if !json.is_null() {
                return Err(format!("{path} should be null (a {ty} can't be written to a file), not {json}"));
            }
            let len = out.bytes.len();
            out.meant[..len].fill(true);
        }
        _ => return Err(format!("{path} is a {ty}, which can't be written to a file")),
    }
    Ok(())
}

/// An integer of type `kind`, checked to fit it.
fn integer(kind: IntKind, json: &Value, path: &str) -> Result<i64, String> {
    let n = json.as_i64().ok_or_else(|| format!("{path} should be a whole number, not {json}"))?;
    if !kind.contains(n) {
        let (lo, hi) = kind.range();
        return Err(format!("{path} is {n}, which doesn't fit {} ({lo}..{hi})", Type::Int(kind)));
    }
    Ok(n)
}

fn string<'a>(json: &'a Value, path: &str) -> Result<&'a str, String> {
    json.as_str().ok_or_else(|| format!("{path} should be a string, not {json}"))
}

/// A character's code in code page 437, which TP's text used.
fn character(c: char, path: &str) -> Result<u8, String> {
    let b = cp437::from_char(c);
    if cp437::to_char(b) != c {
        return Err(format!("{path}: {c:?} isn't a character of code page 437, which Turbo Pascal used"));
    }
    Ok(b)
}

/// The ordinal of a value of the ordinal type `ty`: an enum value by name,
/// a character, a Boolean or a number.
fn ordinal(ty: &Type, json: &Value, path: &str) -> Result<i64, String> {
    match ty {
        Type::Enum(e) => {
            let name = string(json, path)?;
            match e.pascal_names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
                Some(i) => Ok(i as i64),
                None => {
                    Err(format!("{path} is {json}, which isn't one of {ty}'s values ({})", e.pascal_names.join(", ")))
                }
            }
        }
        Type::Char => {
            let s = string(json, path)?;
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Ok(i64::from(character(c, path)?)),
                _ => Err(format!("{path} should be one character, not {json}")),
            }
        }
        Type::Bool => {
            json.as_bool().map(i64::from).ok_or_else(|| format!("{path} should be true or false, not {json}"))
        }
        Type::Int(kind) => integer(*kind, json, path),
        _ => Err(format!("{path}: a {ty} isn't an ordinal type")),
    }
}

fn object<'a>(json: &'a Value, path: &str) -> Result<&'a Map<String, Value>, String> {
    json.as_object().ok_or_else(|| format!("{path} should be an object with the record's fields, not {json}"))
}

/// A record's keys, to check that each is used once, and none is unknown.
struct Keys<'a> {
    fields: &'a Map<String, Value>,
    used: Vec<&'a str>,
    path: &'a str,
}

impl<'a> Keys<'a> {
    fn new(fields: &'a Map<String, Value>, path: &'a str) -> Self {
        Keys { fields, used: Vec::new(), path }
    }

    /// The value of the field `name`, if the JSON has it (in any case, as
    /// Pascal names go).
    fn get(&mut self, name: &str) -> Option<&'a Value> {
        let (key, value) = self.fields.iter().find(|(key, _)| key.eq_ignore_ascii_case(name))?;
        self.used.push(key);
        Some(value)
    }

    fn require(&mut self, name: &str) -> Result<&'a Value, String> {
        self.get(name).ok_or_else(|| format!("{}: the field `{name}` is missing", self.path))
    }

    /// Every key was a field.
    fn done(&self) -> Result<(), String> {
        match self.fields.keys().find(|key| !self.used.contains(&key.as_str())) {
            Some(key) => Err(format!("{}: `{key}` isn't a field here", self.path)),
            None => Ok(()),
        }
    }
}

fn record(analysis: &Analysis, record: &RecordType, keys: &mut Keys, out: &mut Out, path: &str) -> Result<(), String> {
    let mut offset = 0;
    let mut tag = None;
    for (index, field) in record.fixed_fields().iter().enumerate() {
        let size = size_of(&field.ty)?;
        let json = keys.require(&field.name)?;
        value(analysis, &field.ty, json, &mut out.part(offset, size), &format!("{path}.{}", field.name))?;
        if record.variant.as_ref().is_some_and(|part| part.tag == Some(index)) {
            tag = Some(ordinal(&field.ty, json, path)?);
        }
        offset += size;
    }
    if let Some(part) = &record.variant {
        let len = out.bytes.len() - offset;
        variant_part(analysis, part, &record.fields, keys, &mut out.part(offset, len), tag, path)?;
    }
    Ok(())
}

/// The fields of a variant part. With a tag, those of the variant it picks,
/// all required. Without one, any variant's fields may be there (`to-json`
/// writes them all); each is written where it lives, and it's an error only
/// if two of them put different values in the same byte.
fn variant_part(
    analysis: &Analysis,
    part: &VariantPart,
    fields: &[Field],
    keys: &mut Keys,
    area: &mut Out,
    tag: Option<i64>,
    path: &str,
) -> Result<(), String> {
    for variant in &part.variants {
        if let Some(tag) = tag
            && !variant.labels.contains(&tag)
        {
            continue;
        }
        let own = &fields[variant.fields.clone()];
        for (field, &offset) in own.iter().zip(&variant.offsets) {
            let json = if tag.is_some() { Some(keys.require(&field.name)?) } else { keys.get(&field.name) };
            if let Some(json) = json {
                let field_path = format!("{path}.{}", field.name);
                share(analysis, &field.ty, json, area, offset as usize, &field.name, &field_path)?;
            }
        }
        if let Some(nested) = &variant.nested {
            let mut start = own.iter().map(|f| size_of(&f.ty)).sum::<Result<usize, _>>()?;
            let mut nested_tag = None;
            if let Some(index) = nested.tag {
                let field = &fields[index];
                let field_path = format!("{path}.{}", field.name);
                let json = if tag.is_some() { Some(keys.require(&field.name)?) } else { keys.get(&field.name) };
                if let Some(json) = json {
                    share(analysis, &field.ty, json, area, start, &field.name, &field_path)?;
                    nested_tag = Some(ordinal(&field.ty, json, &field_path)?);
                }
                start += size_of(&field.ty)?;
            }
            let len = area.bytes.len() - start;
            // A nested part with a tag that isn't there has nothing to write.
            if nested.tag.is_none() || nested_tag.is_some() {
                variant_part(analysis, nested, fields, keys, &mut area.part(start, len), nested_tag, path)?;
            }
        }
    }
    Ok(())
}

/// Write one field into a variant part's bytes at `offset`, checking that
/// it agrees with what other variants' fields put there.
fn share(
    analysis: &Analysis,
    ty: &Type,
    json: &Value,
    area: &mut Out,
    offset: usize,
    name: &str,
    path: &str,
) -> Result<(), String> {
    let size = size_of(ty)?;
    let mut field = Buffer::new(size);
    value(analysis, ty, json, &mut field.out(), path)?;
    let Buffer { bytes, meant, .. } = field;
    for i in (0..size).filter(|&i| meant[i]) {
        let at = offset + i;
        // A `True` agrees with any byte but 0, which is how TP reads it.
        if *ty == Type::Bool && bytes[i] == 1 && area.meant[at] && area.bytes[at] != 0 {
            continue;
        }
        if area.meant[at] && area.bytes[at] != bytes[i] {
            let other = area.owners[at].as_deref().unwrap_or("another field");
            return Err(format!(
                "{path} and `{other}` share bytes (they're in different variants), but their values don't agree; give only one of them"
            ));
        }
        area.bytes[at] = bytes[i];
        area.meant[at] = true;
        area.owners[at] = Some(name.to_string());
    }
    Ok(())
}

/// An object's fields: its parent's first, then its own.
fn object_fields(
    analysis: &Analysis,
    object: &ObjectType,
    keys: &mut Keys,
    out: &mut Out,
    path: &str,
) -> Result<(), String> {
    let mut offset = 0;
    if let Some(parent) = &object.parent {
        offset = size_of(&Type::Object(parent.clone()))?;
        object_fields(analysis, parent, keys, &mut out.part(0, offset), path)?;
    }
    for field in &object.fields {
        let size = size_of(&field.ty)?;
        let json = keys.require(&field.name)?;
        value(analysis, &field.ty, json, &mut out.part(offset, size), &format!("{path}.{}", field.name))?;
        offset += size;
    }
    Ok(())
}
