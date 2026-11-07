//! Bytes in Turbo Pascal's layout to JSON: one value of a sema `Type`.

use pascrust_runtime::cp437;
use pascrust_runtime::io::real48;
use serde_json::{Map, Number, Value};

use crate::sema::types::{Field, ObjectType, RealKind, RecordType, VariantPart, set_bytes, tp_size};
use crate::sema::{Analysis, IntKind, Type};

/// The value of type `ty` in `bytes` (exactly its size). `path` says where
/// it is (`[3].address.zip`), for messages.
pub fn value(analysis: &Analysis, ty: &Type, bytes: &[u8], path: &str) -> Result<Value, String> {
    Ok(match ty {
        Type::Int(kind) => Value::from(integer(*kind, bytes)),
        // TP stores `True` as 1, but any byte other than 0 is true, as it
        // is to TP (a record that was never cleared may hold other values).
        Type::Bool => Value::Bool(bytes[0] != 0),
        Type::Char => Value::String(cp437::to_char(bytes[0]).to_string()),
        Type::Str { cap } => {
            // The length byte, then the characters; the bytes after them
            // are whatever was there before, which TP never cleared.
            let len = usize::from(bytes[0].min(*cap));
            Value::String(bytes[1..=len].iter().map(|&b| cp437::to_char(b)).collect())
        }
        Type::Real(RealKind::Real) => real(bytes.try_into().expect("a Real is 6 bytes")),
        Type::Real(kind) => {
            return Err(format!(
                "{path} is a {kind:?}: PascRust reads only 6-byte `Real`s from files for now, not {kind:?}s"
            ));
        }
        Type::Enum(e) => {
            let ordinal = integer(if bytes.len() == 1 { IntKind::Byte } else { IntKind::Word }, bytes);
            match e.pascal_names.get(ordinal as usize) {
                Some(name) => Value::String(name.clone()),
                None => return Err(format!("{path} is {ordinal}, which isn't a value of {ty}")),
            }
        }
        Type::Set(elem, _) => set(elem, ty, bytes),
        Type::Array(array) => {
            let size = size_of(&array.elem)?;
            let elements = bytes
                .chunks(size)
                .enumerate()
                .map(|(i, element)| value(analysis, &array.elem, element, &format!("{path}[{i}]")))
                .collect::<Result<Vec<_>, _>>()?;
            Value::Array(elements)
        }
        Type::Record(record) => Value::Object(self::record(analysis, record, bytes, path)?),
        Type::Object(object) => {
            let mut map = Map::new();
            self::object(analysis, object, bytes, path, &mut map)?;
            Value::Object(map)
        }
        // An address, or a file variable's handle, means nothing outside the
        // run that wrote it.
        Type::Pointer(_) | Type::AnyPointer | Type::PChar | Type::Proc(_) | Type::Text | Type::File(_) => Value::Null,
        _ => return Err(format!("{path} is a {ty}, which can't be read from a file")),
    })
}

/// A value's size in bytes, in TP's layout.
fn size_of(ty: &Type) -> Result<usize, String> {
    tp_size(ty).map(|size| size as usize).ok_or_else(|| format!("{ty} can't be in a typed file"))
}

/// A little-endian integer of type `kind`.
fn integer(kind: IntKind, bytes: &[u8]) -> i64 {
    match kind {
        IntKind::ShortInt => i64::from(bytes[0] as i8),
        IntKind::Byte => i64::from(bytes[0]),
        IntKind::Integer => i64::from(i16::from_le_bytes([bytes[0], bytes[1]])),
        IntKind::Word => i64::from(u16::from_le_bytes([bytes[0], bytes[1]])),
        IntKind::LongInt => i64::from(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
    }
}

/// A 6-byte `Real` as the shortest decimal that is stored as the same
/// bytes: `0.1` rather than `0.10000000000002274`, which is exactly what
/// the 39 bits of a `Real` hold.
fn real(bytes: [u8; 6]) -> Value {
    let exact = real48::to_f64(bytes);
    let shortest = (1..=17)
        .filter_map(|digits| format!("{exact:.*e}", digits - 1).parse::<f64>().ok())
        .find(|&x| real48::from_f64(x) == bytes)
        .unwrap_or(exact);
    Number::from_f64(shortest).map_or(Value::Null, Value::Number)
}

/// A set: TP keeps a bit per possible element, in the bytes from its lowest
/// value's to its highest's (see `set_bytes`). The JSON lists the elements.
fn set(elem: &Type, ty: &Type, bytes: &[u8]) -> Value {
    let (first, _) = set_bytes(ty);
    let mut elements = Vec::new();
    for (i, byte) in bytes.iter().enumerate() {
        for bit in 0..8 {
            if byte & (1 << bit) != 0 {
                let ordinal = (first + i) * 8 + bit;
                elements.push(element(elem, ordinal as i64));
            }
        }
    }
    Value::Array(elements)
}

/// A set's element, from its ordinal.
fn element(elem: &Type, ordinal: i64) -> Value {
    match elem {
        Type::Char => Value::String(cp437::to_char(ordinal as u8).to_string()),
        Type::Bool => Value::Bool(ordinal != 0),
        Type::Enum(e) => {
            e.pascal_names.get(ordinal as usize).map_or(Value::from(ordinal), |n| Value::String(n.clone()))
        }
        _ => Value::from(ordinal),
    }
}

/// A record: its fixed fields in order, then its variant part.
fn record(analysis: &Analysis, record: &RecordType, bytes: &[u8], path: &str) -> Result<Map<String, Value>, String> {
    let mut map = Map::new();
    let mut offset = 0;
    // The tag's value (its ordinal), which picks the variant.
    let mut tag = None;
    for (index, field) in record.fixed_fields().iter().enumerate() {
        let size = size_of(&field.ty)?;
        let field_bytes = &bytes[offset..offset + size];
        map.insert(field.name.clone(), value(analysis, &field.ty, field_bytes, &format!("{path}.{}", field.name))?);
        if record.variant.as_ref().is_some_and(|part| part.tag == Some(index)) {
            tag = Some(ordinal(&field.ty, field_bytes));
        }
        offset += size;
    }
    if let Some(part) = &record.variant {
        variant_part(analysis, part, &record.fields, &bytes[offset..], tag, path, &mut map)?;
    }
    Ok(map)
}

/// The fields of a variant part, whose variants share `area`. With a tag
/// (`tag` is its value), only the variant it picks; without one, every
/// variant, each read from the same bytes, since nothing says which is meant.
fn variant_part(
    analysis: &Analysis,
    part: &VariantPart,
    fields: &[Field],
    area: &[u8],
    tag: Option<i64>,
    path: &str,
    map: &mut Map<String, Value>,
) -> Result<(), String> {
    for variant in &part.variants {
        if let Some(tag) = tag
            && !variant.labels.contains(&tag)
        {
            continue;
        }
        let own = &fields[variant.fields.clone()];
        for (field, &offset) in own.iter().zip(&variant.offsets) {
            let (start, size) = (offset as usize, size_of(&field.ty)?);
            let field_path = format!("{path}.{}", field.name);
            map.insert(field.name.clone(), value(analysis, &field.ty, &area[start..start + size], &field_path)?);
        }
        // A nested part starts after the variant's own fields: its tag
        // (if it has one), then its variants.
        if let Some(nested) = &variant.nested {
            let mut start = own.iter().map(|f| size_of(&f.ty)).sum::<Result<usize, _>>()?;
            let mut nested_tag = None;
            if let Some(index) = nested.tag {
                let field = &fields[index];
                let size = size_of(&field.ty)?;
                let tag_bytes = &area[start..start + size];
                let field_path = format!("{path}.{}", field.name);
                map.insert(field.name.clone(), value(analysis, &field.ty, tag_bytes, &field_path)?);
                nested_tag = Some(ordinal(&field.ty, tag_bytes));
                start += size;
            }
            variant_part(analysis, nested, fields, &area[start..], nested_tag, path, map)?;
        }
    }
    Ok(())
}

/// An ordinal value's number (a tag's), from its bytes.
fn ordinal(ty: &Type, bytes: &[u8]) -> i64 {
    match ty {
        Type::Int(kind) => integer(*kind, bytes),
        // Any byte but 0 is `True`, as for the value itself.
        Type::Bool => i64::from(bytes[0] != 0),
        _ if bytes.len() == 2 => integer(IntKind::Word, bytes),
        _ => i64::from(bytes[0]),
    }
}

/// An object's fields: its parent's first, then its own, as TP lays it out.
/// TP's hidden VMT field, if this type adds it, comes after them, so there's
/// nothing more to read: `tp_size` counts its 2 bytes.
fn object(
    analysis: &Analysis,
    object: &ObjectType,
    bytes: &[u8],
    path: &str,
    map: &mut Map<String, Value>,
) -> Result<(), String> {
    let mut offset = 0;
    if let Some(parent) = &object.parent {
        offset = size_of(&Type::Object(parent.clone()))?;
        self::object(analysis, parent, &bytes[..offset], path, map)?;
    }
    for field in &object.fields {
        let size = size_of(&field.ty)?;
        let field_path = format!("{path}.{}", field.name);
        map.insert(field.name.clone(), value(analysis, &field.ty, &bytes[offset..offset + size], &field_path)?);
        offset += size;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reals_print_as_their_shortest_decimal() {
        for x in [0.1, 2.5e7, 123.456, 2930802.0, -1.5e-10] {
            assert_eq!(real(real48::from_f64(x)), Value::Number(Number::from_f64(x).unwrap()), "{x}");
        }
    }
}
