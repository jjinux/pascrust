//! `Write` and `Writeln`.
//!
//! Pascal's `Write` is not an ordinary procedure: it takes any number of
//! arguments of almost any type, and each argument may carry a field width
//! and a number of decimals, as in `writeln('Total: ', total:5, avg:8:2)`.
//! Rust functions can't do that, so the runtime offers a macro
//! (`pwriteln!("Total: ", w(total, 5), wd(avg, 8, 2))`) plus a trait,
//! [`PasWrite`], that every printable type implements.

/// A value that Pascal's `Write` can print.
pub trait PasWrite {
    /// Append this value's text, as Turbo Pascal would print it with no field
    /// width, to `out`. Output is bytes, not UTF-8, because Pascal `Char`s are
    /// bytes.
    fn pas_write(&self, out: &mut Vec<u8>);

    /// Print with a field width (`x:width`). The default right-aligns the
    /// unformatted text; never truncates, which is what Turbo Pascal does.
    fn pas_write_width(&self, width: i32, out: &mut Vec<u8>) {
        let mut text = Vec::new();
        self.pas_write(&mut text);
        pad_left(&text, width, out);
    }

    /// Print with a width and a number of decimals (`x:width:decimals`). Only
    /// real numbers really support this; other types ignore `decimals`.
    fn pas_write_decimals(&self, width: i32, _decimals: i32, out: &mut Vec<u8>) {
        self.pas_write_width(width, out);
    }
}

fn pad_left(text: &[u8], width: i32, out: &mut Vec<u8>) {
    let width = width.max(0) as usize;
    if text.len() < width {
        out.resize(out.len() + width - text.len(), b' ');
    }
    out.extend_from_slice(text);
}

/// A value plus its `:width:decimals` formatting, built by [`w`] and [`wd`].
pub struct Formatted<T> {
    value: T,
    width: i32,
    decimals: Option<i32>,
}

/// Pascal `x:width`.
pub fn w<T: PasWrite>(value: T, width: impl Into<i32>) -> Formatted<T> {
    Formatted { value, width: width.into(), decimals: None }
}

/// Pascal `x:width:decimals`.
pub fn wd<T: PasWrite>(value: T, width: impl Into<i32>, decimals: impl Into<i32>) -> Formatted<T> {
    Formatted { value, width: width.into(), decimals: Some(decimals.into()) }
}

impl<T: PasWrite> PasWrite for Formatted<T> {
    fn pas_write(&self, out: &mut Vec<u8>) {
        match self.decimals {
            None => self.value.pas_write_width(self.width, out),
            Some(d) => self.value.pas_write_decimals(self.width, d, out),
        }
    }
}

// A "blanket impl": any reference to something printable is printable too.
// This lets the macros take `&arg` uniformly, whether `arg` is a value, a
// `&str` literal or a byte string.
impl<T: PasWrite + ?Sized> PasWrite for &T {
    fn pas_write(&self, out: &mut Vec<u8>) {
        (**self).pas_write(out)
    }
    fn pas_write_width(&self, width: i32, out: &mut Vec<u8>) {
        (**self).pas_write_width(width, out)
    }
    fn pas_write_decimals(&self, width: i32, decimals: i32, out: &mut Vec<u8>) {
        (**self).pas_write_decimals(width, decimals, out)
    }
}

/// String literals made only of ASCII are emitted as Rust `&str`s.
impl PasWrite for str {
    fn pas_write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
}

/// String literals with non-ASCII bytes (code page 437) are emitted as Rust
/// byte strings, `b"..."`, whose type is `&[u8; N]`. The `const N: usize`
/// parameter makes this one impl cover arrays of every length.
impl<const N: usize> PasWrite for [u8; N] {
    fn pas_write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

impl PasWrite for [u8] {
    fn pas_write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

/// Pascal's `Char` is a byte, so a `u8` prints as a character. (Pascal's
/// `Byte` is a `u8` too; codegen widens bytes to `i32` before printing them
/// so they come out as numbers.)
impl PasWrite for u8 {
    fn pas_write(&self, out: &mut Vec<u8>) {
        out.push(*self);
    }
}

impl PasWrite for bool {
    fn pas_write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(if *self { b"TRUE" } else { b"FALSE" });
    }
}

// The same impl for every integer type. `macro_rules!` is the simplest way
// to stamp out near-identical impls without writing them by hand.
macro_rules! impl_pas_write_int {
    ($($t:ty),*) => {
        $(
            impl PasWrite for $t {
                fn pas_write(&self, out: &mut Vec<u8>) {
                    out.extend_from_slice(self.to_string().as_bytes());
                }
            }
        )*
    };
}
impl_pas_write_int!(i8, i16, i32, i64, u16, u32, u64);

/// Turbo Pascal prints a `Real` with no width in scientific notation, 17
/// characters wide: ` 1.2345000000E+02`.
const DEFAULT_REAL_WIDTH: i32 = 17;

impl PasWrite for f64 {
    fn pas_write(&self, out: &mut Vec<u8>) {
        self.pas_write_width(DEFAULT_REAL_WIDTH, out);
    }

    /// `x:width` prints in scientific notation, using as many digits as fit.
    fn pas_write_width(&self, width: i32, out: &mut Vec<u8>) {
        // The fixed parts are: sign or space, one digit, '.', 'E', the
        // exponent sign and two exponent digits, which is 7 characters.
        let digits = (width - 7).clamp(1, 10) as usize;
        let text = format_scientific(*self, digits);
        pad_left(text.as_bytes(), width, out);
    }

    /// `x:width:decimals` prints in fixed-point notation.
    fn pas_write_decimals(&self, width: i32, decimals: i32, out: &mut Vec<u8>) {
        let text = format!("{:.*}", decimals.max(0) as usize, self);
        pad_left(text.as_bytes(), width, out);
    }
}

impl PasWrite for crate::string::PString {
    fn pas_write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
}

impl PasWrite for f32 {
    fn pas_write(&self, out: &mut Vec<u8>) {
        f64::from(*self).pas_write(out)
    }
    fn pas_write_width(&self, width: i32, out: &mut Vec<u8>) {
        f64::from(*self).pas_write_width(width, out)
    }
    fn pas_write_decimals(&self, width: i32, decimals: i32, out: &mut Vec<u8>) {
        f64::from(*self).pas_write_decimals(width, decimals, out)
    }
}

/// Format like Turbo Pascal: ` 1.2340000000E+02` or `-1.2340000000E-05`.
fn format_scientific(x: f64, digits: usize) -> String {
    // Rust's `{:e}` gives `1.2340000000e2`; reshape the exponent.
    let s = format!("{:.*e}", digits, x.abs());
    let (mantissa, exp) = s.split_once('e').expect("`{:e}` always has an exponent");
    let exp: i32 = exp.parse().expect("exponent is an integer");
    let sign = if x.is_sign_negative() { '-' } else { ' ' };
    let exp_sign = if exp < 0 { '-' } else { '+' };
    format!("{sign}{mantissa}E{exp_sign}{:02}", exp.abs())
}

/// Pascal `Write(a, b, ...)` to standard output.
///
/// `$crate` always refers to this runtime crate, wherever the macro is used,
/// so generated code doesn't need to import the helper functions.
#[macro_export]
macro_rules! pwrite {
    ($($arg:expr),* $(,)?) => {{
        let mut buf: ::std::vec::Vec<u8> = ::std::vec::Vec::new();
        $( $crate::io::PasWrite::pas_write(&$arg, &mut buf); )*
        $crate::io::write_stdout(&buf);
    }};
}

/// Pascal `Writeln(a, b, ...)`: like [`pwrite!`] followed by a line break.
#[macro_export]
macro_rules! pwriteln {
    ($($arg:expr),* $(,)?) => {{
        let mut buf: ::std::vec::Vec<u8> = ::std::vec::Vec::new();
        $( $crate::io::PasWrite::pas_write(&$arg, &mut buf); )*
        buf.push(b'\n');
        $crate::io::write_stdout(&buf);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: impl PasWrite) -> String {
        let mut out = Vec::new();
        value.pas_write(&mut out);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn integers_and_widths() {
        assert_eq!(text(42i16), "42");
        assert_eq!(text(w(42i16, 5)), "   42");
        assert_eq!(text(w(-42i32, 2)), "-42");
    }

    #[test]
    fn booleans_and_chars() {
        assert_eq!(text(true), "TRUE");
        assert_eq!(text(w(false, 7)), "  FALSE");
        assert_eq!(text(b'x'), "x");
    }

    #[test]
    fn reals() {
        assert_eq!(text(123.4f64), " 1.2340000000E+02");
        assert_eq!(text(-0.00001234f64), "-1.2340000000E-05");
        assert_eq!(text(wd(2.71666f64, 8, 2)), "    2.72");
        assert_eq!(text(wd(-2.5f64, 0, 1)), "-2.5");
    }
}
