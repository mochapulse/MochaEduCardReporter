//! Explicit emulation of the Python semantics the engine relies on
//! (`PLAN_REDO_CODE.md` section 3.3): `float()`, `f"{x:.1f}"`, `f"{x:g}"`,
//! `str.strip()` / `str.isspace()` and `int(float(text))`.

/// Python `str.isspace()` for one character (used by `str.strip()`, by the
/// `\s` class of `re`, and by `float()`'s whitespace trimming).
///
/// Mirrors the Unicode White_Space property plus U+001C..U+001F, which
/// Python treats as whitespace but Rust's `char::is_whitespace` does not.
pub fn py_isspace(c: char) -> bool {
    c.is_whitespace() || ('\u{1C}'..='\u{1F}').contains(&c)
}

/// Python `str.strip()` with no argument (strips [`py_isspace`] characters
/// from both ends).
pub fn py_strip(s: &str) -> &str {
    s.trim_matches(py_isspace)
}

/// First code point (digit value 0) of every run of ten Unicode decimal digits.
pub const DECIMAL_ZERO_STARTS: &[u32] = &[
    0x30, 0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66,
    0xDE6, 0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90,
    0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10,
    0x104A0, 0x10D30, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0, 0x11650,
    0x116C0, 0x11730, 0x118E0, 0x11950, 0x11C50, 0x11D50, 0x11DA0, 0x11F50, 0x16A60, 0x16AC0,
    0x16B50, 0x1D7CE, 0x1D7D8, 0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0, 0x1E950,
    0x1FBF0,
];

/// Decimal digit value of `c` as Python's `float()` sees it
/// (`unicodedata.decimal`): ASCII digits and every Unicode Nd digit.
fn decimal_value(c: char) -> Option<u8> {
    let cp = c as u32;
    // Index of the last run start <= cp; the run covers ten code points.
    let idx = DECIMAL_ZERO_STARTS.partition_point(|&s| s <= cp);
    let start = *DECIMAL_ZERO_STARTS.get(idx.checked_sub(1)?)?;
    let offset = cp - start;
    if offset < 10 {
        u8::try_from(offset).ok()
    } else {
        None
    }
}

/// Consume `digit (_? digit)*` starting at `*i`; return whether at least one
/// digit was consumed. An underscore is accepted only between two digits
/// (CPython `_Py_string_to_number_with_underscores`).
fn scan_digits(b: &[u8], i: &mut usize) -> bool {
    if !b.get(*i).is_some_and(u8::is_ascii_digit) {
        return false;
    }
    loop {
        *i += 1;
        match (b.get(*i), b.get(*i + 1)) {
            (Some(d), _) if d.is_ascii_digit() => {}
            (Some(b'_'), Some(d)) if d.is_ascii_digit() => *i += 1,
            _ => return true,
        }
    }
}

/// Python `float(text)` for a `str`: returns `Some` exactly when Python does
/// not raise `ValueError` (py lines 128, 150 and others).
///
/// Emulated rules: surrounding whitespace is ignored (ASCII `" \t\n\v\f\r"`
/// and non-ASCII [`py_isspace`] characters, but NOT U+001C..U+001F); Unicode
/// decimal digits count as digits; an optional sign; `inf`, `infinity`, `nan`
/// (any case, signed); digits with single underscores between digits;
/// `.5`, `5.`, exponents. Rejected: `0x10`, `_1`, `1__0`, `4,5`, empty, other
/// non-ASCII characters, interior whitespace. Overflowing literals give
/// `inf` like Python does.
pub fn py_float(text: &str) -> Option<f64> {
    // `_PyUnicode_TransformDecimalAndSpaceToASCII`: whitespace -> ' ',
    // Nd digits -> ASCII, any other non-ASCII char -> '?' (always invalid).
    let mut t = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            // ASCII is copied unchanged, so U+001C..U+001F are NOT trimmed
            // by `float()` even though `str.isspace()` accepts them.
            t.push(c);
        } else if py_isspace(c) {
            t.push(' ');
        } else {
            // Any other non-ASCII char becomes '?', which is always invalid.
            t.push(char::from(b'0' + decimal_value(c)?));
        }
    }
    // CPython trims with the ASCII-only `Py_ISSPACE`.
    let t = t.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\u{B}' | '\u{C}' | '\r'));
    let b = t.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let rest = &t[i..];
    if ["inf", "infinity", "nan"]
        .iter()
        .any(|w| rest.eq_ignore_ascii_case(w))
    {
        return t.parse::<f64>().ok();
    }
    let int_digits = scan_digits(b, &mut i);
    let mut frac_digits = false;
    if b.get(i) == Some(&b'.') {
        i += 1;
        frac_digits = scan_digits(b, &mut i);
    }
    if !int_digits && !frac_digits {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        if !scan_digits(b, &mut i) {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    t.replace('_', "").parse::<f64>().ok()
}

/// Python `f"{x:.1f}"` (py lines 134 and others): round the exact binary value
/// to one decimal (ties-to-even on the exact value), keep the sign of `-0.0`
/// and of small negatives (`-0.04` gives `"-0.0"`), and print `inf`, `-inf`,
/// `nan` as Python does.
pub fn fmt_1f(x: f64) -> String {
    if x.is_nan() {
        "nan".to_string()
    } else if x.is_infinite() {
        if x < 0.0 { "-inf" } else { "inf" }.to_string()
    } else {
        format!("{x:.1}")
    }
}

/// Python `f"{x:g}"` (py line 275): six significant digits, trailing zeros
/// and a dangling point removed, scientific notation (`1e+16`, `1e-05`, at
/// least two exponent digits) when the decimal exponent is below -4 or at
/// least 6. `nan`/`inf` as Python prints them; `-0.0` gives `"-0"`.
pub fn fmt_g(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    let sign = if x.is_sign_negative() { "-" } else { "" };
    if x == 0.0 {
        return format!("{sign}0");
    }
    let sci = format!("{:.5e}", x.abs());
    let (mant, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let body = if (-4..6).contains(&exp) {
        let prec = usize::try_from(5 - exp).unwrap_or(0);
        strip_zeros(format!("{:.*}", prec, x.abs()))
    } else {
        let m = strip_zeros(mant.to_string());
        let e_sign = if exp < 0 { '-' } else { '+' };
        format!("{m}e{e_sign}{:02}", exp.abs())
    };
    format!("{sign}{body}")
}

/// Remove trailing zeros after a decimal point, and the point itself if
/// nothing remains after it.
fn strip_zeros(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Python `str(int(float(text)))` (parser, py lines 701 and 710): `None` when
/// `float` raises `ValueError` or `int` raises (`inf` -> OverflowError,
/// `nan` -> ValueError); otherwise the exact decimal expansion of the
/// truncated value (arbitrary size, `-0.5` gives `"0"`).
pub fn py_int_from_float_str(text: &str) -> Option<String> {
    let f = py_float(text)?;
    if !f.is_finite() {
        return None;
    }
    let t = f.trunc();
    if t == 0.0 {
        return Some("0".to_string());
    }
    Some(format!("{t:.0}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isspace_and_strip() {
        for c in [
            '\u{1C}', '\u{1F}', ' ', '\u{85}', '\u{A0}', '\u{3000}', '\u{2029}',
        ] {
            assert!(py_isspace(c), "{c:?}");
        }
        for c in ['a', '\u{200B}', '\u{FEFF}', '_'] {
            assert!(!py_isspace(c), "{c:?}");
        }
        assert_eq!(py_strip("\u{1C} a b\u{A0}\n"), "a b");
    }

    #[test]
    fn py_float_accepts() {
        let cases: &[(&str, f64)] = &[
            ("1", 1.0),
            (" 1.5 ", 1.5),
            ("-.5", -0.5),
            ("5.", 5.0),
            ("+1e3", 1000.0),
            ("1_0", 10.0),
            ("1_0.0_1", 10.01),
            ("1e1_0", 1e10),
            ("\u{663}", 3.0),
            ("\u{FF11}\u{FF12}", 12.0),
            ("1\u{3000}", 1.0),
            ("1\n", 1.0),
            ("1e999", f64::INFINITY),
            ("-INF", f64::NEG_INFINITY),
            ("Infinity", f64::INFINITY),
            ("+inf", f64::INFINITY),
        ];
        for (s, v) in cases {
            assert_eq!(py_float(s), Some(*v), "{s:?}");
        }
        assert!(py_float("NaN").is_some_and(f64::is_nan));
        assert!(py_float("-nan").is_some_and(f64::is_nan));
        assert!(py_float("-0").is_some_and(|f| f == 0.0 && f.is_sign_negative()));
    }

    #[test]
    fn py_float_rejects() {
        for s in [
            "",
            " ",
            "0x10",
            "_1",
            "1_",
            "1__0",
            "4,5",
            "1._5",
            "1_.5",
            "1e_5",
            "1e",
            "e5",
            ".",
            "+",
            "--1",
            "1 2",
            "infin",
            "i_nf",
            "1\u{1C}",
            "\u{1C}1",
            "1f",
            "\u{BD}",
            "\u{2460}",
            "1\u{0}",
            "\u{661}\u{2460}",
            "1e5.5",
            "0b1",
            "1,000",
        ] {
            assert_eq!(py_float(s), None, "{s:?}");
        }
    }

    #[test]
    fn one_decimal() {
        let cases = [
            (0.25, "0.2"),
            (0.35, "0.3"),
            (2.675, "2.7"),
            (-0.04, "-0.0"),
            (-0.0, "-0.0"),
            (0.05, "0.1"),
            (15.0, "15.0"),
            (1234567.89, "1234567.9"),
            (1e22, "10000000000000000000000.0"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
            (f64::NAN, "nan"),
        ];
        for (x, s) in cases {
            assert_eq!(fmt_1f(x), s, "{x:?}");
        }
    }

    #[test]
    fn general_format() {
        let cases = [
            (0.0, "0"),
            (-0.0, "-0"),
            (20.0, "20"),
            (12.5, "12.5"),
            (0.25, "0.25"),
            (1e16, "1e+16"),
            (1e-5, "1e-05"),
            (0.0001, "0.0001"),
            (123456.0, "123456"),
            (1234567.0, "1.23457e+06"),
            (999999.5, "1e+06"),
            (100000.0, "100000"),
            (1e100, "1e+100"),
            (1.5e-7, "1.5e-07"),
            (0.1 + 0.2, "0.3"),
            (-2.5e-10, "-2.5e-10"),
            (1234567.89, "1.23457e+06"),
            (f64::INFINITY, "inf"),
            (f64::NAN, "nan"),
        ];
        for (x, s) in cases {
            assert_eq!(fmt_g(x), s, "{x:?}");
        }
    }

    #[test]
    fn int_of_float() {
        assert_eq!(py_int_from_float_str("3"), Some("3".into()));
        assert_eq!(py_int_from_float_str("3.9"), Some("3".into()));
        assert_eq!(py_int_from_float_str("-0.5"), Some("0".into()));
        assert_eq!(py_int_from_float_str("-2.5"), Some("-2".into()));
        assert_eq!(py_int_from_float_str("1e3"), Some("1000".into()));
        assert_eq!(py_int_from_float_str("inf"), None);
        assert_eq!(py_int_from_float_str("nan"), None);
        assert_eq!(py_int_from_float_str("x"), None);
        assert_eq!(
            py_int_from_float_str("1e22"),
            Some("10000000000000000000000".into())
        );
    }
}
