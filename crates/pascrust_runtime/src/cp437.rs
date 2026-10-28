//! Code page 437, the character set of the IBM PC and so of every Turbo
//! Pascal program: ASCII plus accented letters, Greek, maths symbols and the
//! famous box-drawing characters in bytes 128..255. Pascal `Char`s are
//! these bytes; terminals today speak Unicode, so output is translated.

/// The Unicode character for each byte from 128 to 255.
const HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', //
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', //
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', //
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐', //
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧', //
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀', //
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', //
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

/// The character a byte stands for.
pub fn to_char(b: u8) -> char {
    if b < 128 { b as char } else { HIGH[(b - 128) as usize] }
}

/// The byte for a character, or `?` if code page 437 doesn't have it.
pub fn from_char(c: char) -> u8 {
    if c.is_ascii() {
        return c as u8;
    }
    HIGH.iter().position(|&h| h == c).map_or(b'?', |i| i as u8 + 128)
}

/// Translate Pascal output bytes to UTF-8 for the terminal.
pub fn to_utf8(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for &b in bytes {
        if b < 128 {
            out.push(b);
        } else {
            let mut buf = [0; 4];
            out.extend_from_slice(to_char(b).encode_utf8(&mut buf).as_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for b in 0..=255u8 {
            if b != b'?' {
                assert_eq!(from_char(to_char(b)), b);
            }
        }
        assert_eq!(to_char(0xc9), '╔');
    }
}
