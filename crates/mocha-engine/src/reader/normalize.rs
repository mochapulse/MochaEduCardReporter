//! Emulation of `pd.read_excel(...)` followed by
//! `df.to_csv(index=False, float_format="%.1f")` (py lines 1732-1745 and
//! 357-361).
//!
//! Input is the list-of-lists pandas' Excel readers hand to its Python
//! `TextParser` (header row first, every row the same width). Output is the
//! exact CSV text pandas writes. Confirmed rules are listed on each function.

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::pycompat::fmt_1f;

/// One cell as pandas' Excel readers deliver it to the text parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Empty cell or empty string (`""` in pandas' list-of-lists).
    Blank,
    /// Error cell (`np.nan`).
    Nan,
    /// Text.
    Str(String),
    /// Whole number (`int`), including integral floats.
    Int(i128),
    /// Whole number beyond `i128` (decimal text, original value).
    Huge(String, f64),
    /// Non-integral (or non-finite) float.
    Float(f64),
    /// Boolean.
    Bool(bool),
    /// `datetime.datetime`.
    DateTime(NaiveDateTime),
    /// `datetime.time`.
    Time(NaiveTime),
}

/// pandas' default NA strings (`STR_NA_VALUES`), applied to whole cells.
const NA_VALUES: &[&str] = &[
    "", "#N/A", "#N/A N/A", "#NA", "-1.#IND", "-1.#QNAN", "-NaN", "-nan", "1.#IND", "1.#QNAN",
    "<NA>", "N/A", "NA", "NULL", "NaN", "None", "n/a", "nan", "null",
];

/// Python `repr(float)` / `str(float)`: shortest round-trip digits, scientific
/// notation when the decimal exponent is below -4 or at least 16.
pub fn py_repr_float(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    let s = format!("{x:e}");
    let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (sign, mant) = match mant.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mant),
    };
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let body = if (-4..16).contains(&exp) {
        if exp >= 0 {
            let int_len = exp as usize + 1;
            if digits.len() <= int_len {
                format!("{}{}.0", digits, "0".repeat(int_len - digits.len()))
            } else {
                format!("{}.{}", &digits[..int_len], &digits[int_len..])
            }
        } else {
            format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
        }
    } else {
        let rest = &digits[1..];
        let dot = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let esign = if exp < 0 { '-' } else { '+' };
        format!("{}{}e{}{:02}", &digits[..1], dot, esign, exp.abs())
    };
    format!("{sign}{body}")
}

/// Python `str(datetime.datetime)`.
fn str_datetime(dt: &NaiveDateTime) -> String {
    let base = dt.format("%Y-%m-%d %H:%M:%S").to_string();
    let micro = dt.nanosecond() / 1000;
    if micro != 0 {
        format!("{base}.{micro:06}")
    } else {
        base
    }
}

/// Python `str(datetime.time)`.
fn str_time(t: &NaiveTime) -> String {
    let base = t.format("%H:%M:%S").to_string();
    let micro = t.nanosecond() / 1000;
    if micro != 0 {
        format!("{base}.{micro:06}")
    } else {
        base
    }
}

/// Text of a `datetime64[us]` array as `to_csv` writes it: date only when
/// every value is at midnight, otherwise `H:M:S` plus a fraction of 3 digits
/// when every value is a whole number of milliseconds, else 6 digits.
fn format_datetime64(values: &[Option<NaiveDateTime>]) -> Vec<String> {
    let present: Vec<&NaiveDateTime> = values.iter().flatten().collect();
    let dates_only = present.iter().all(|d| d.time() == NaiveTime::MIN);
    let any_micro = present.iter().any(|d| (d.nanosecond() / 1000) % 1000 != 0);
    let any_milli = present.iter().any(|d| d.nanosecond() / 1000 != 0);
    values
        .iter()
        .map(|v| match v {
            None => String::new(),
            Some(d) if dates_only => d.format("%Y-%m-%d").to_string(),
            Some(d) => {
                let base = d.format("%Y-%m-%d %H:%M:%S").to_string();
                let micro = d.nanosecond() / 1000;
                if any_micro {
                    format!("{base}.{micro:06}")
                } else if any_milli {
                    format!("{base}.{:03}", micro / 1000)
                } else {
                    base
                }
            }
        })
        .collect()
}

/// pandas' `floatify` (used by `maybe_convert_numeric` for text cells):
/// surrounding C whitespace ignored, optional sign, `inf`/`infinity` in any
/// case, decimal digits with optional point and `e`/`E` exponent. Returns the
/// value and whether the text looks like an integer (no point, no exponent).
fn floatify(s: &str) -> Option<(f64, bool)> {
    let t = s.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r'));
    if t.is_empty() {
        return None;
    }
    let (neg, body) = match t.as_bytes()[0] {
        b'-' => (true, &t[1..]),
        b'+' => (false, &t[1..]),
        _ => (false, t),
    };
    let lower = body.to_ascii_lowercase();
    if lower == "inf" || lower == "infinity" {
        return Some((
            if neg {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            },
            false,
        ));
    }
    let b = body.as_bytes();
    let mut i = 0;
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    let mut maybe_int = true;
    if i < b.len() && b[i] == b'.' {
        maybe_int = false;
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        maybe_int = false;
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    let v: f64 = body.parse().ok()?;
    Some((if neg { -v } else { v }, maybe_int))
}

/// Parse integer text as Python `int()` would, as a [`Cell`].
fn int_cell_from_text(s: &str) -> Cell {
    let t = s.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r'));
    match t.parse::<i128>() {
        Ok(v) => Cell::Int(v),
        Err(_) => {
            let neg = t.starts_with('-');
            let digits = t.trim_start_matches(['+', '-']).trim_start_matches('0');
            let v: f64 = t.parse().unwrap_or(f64::INFINITY);
            let text = if digits.is_empty() {
                "0".to_string()
            } else {
                digits.to_string()
            };
            Cell::Huge(if neg { format!("-{text}") } else { text }, v)
        }
    }
}

/// Numeric item as seen by `lib.maybe_convert_numeric`.
enum Num {
    Null,
    /// Integer text `-0` (an int that is `-0.0` as a float).
    NegZero,
    Int(i128),
    Huge(f64),
    Float(f64),
    Bool(bool),
}

/// Outcome of [`numeric_attempt`].
enum Attempt {
    /// A numeric or boolean column, already printed.
    Done(Vec<String>),
    /// pandas raised and fell back to the `object` path (`sanitize_objects`).
    Object,
    /// `check_uint64_conflict`: the untouched values are returned, so NA text
    /// is not replaced and equal values are not collapsed.
    Raw,
}

/// Emulates `lib.maybe_convert_numeric(values, na_values, False)`: `None`
/// when pandas gives up (any non-numeric text, datetime or time, a negative
/// number next to a value above int64, a huge integer without floats), else
/// the printed column text (`float_format="%.1f"` applies to float64 columns).
fn numeric_attempt(vals: &[Cell]) -> Attempt {
    let mut items = Vec::with_capacity(vals.len());
    for v in vals {
        items.push(match v {
            Cell::Blank | Cell::Nan => Num::Null,
            Cell::Str(s) if NA_VALUES.contains(&s.as_str()) => Num::Null,
            Cell::Str(s) => {
                let Some((f, maybe_int)) = floatify(s) else {
                    return Attempt::Object;
                };
                if maybe_int && f == 0.0 && f.is_sign_negative() {
                    Num::NegZero
                } else if maybe_int {
                    match int_cell_from_text(s) {
                        Cell::Int(i) => Num::Int(i),
                        _ => Num::Huge(f),
                    }
                } else {
                    Num::Float(f)
                }
            }
            Cell::Int(i) => Num::Int(*i),
            Cell::Huge(_, f) => Num::Huge(*f),
            Cell::Float(f) => Num::Float(*f),
            Cell::Bool(b) => Num::Bool(*b),
            Cell::DateTime(_) | Cell::Time(_) => return Attempt::Object,
        });
    }
    let mut any_float = false;
    let mut any_null = false;
    let mut any_int = false;
    let mut any_uint = false;
    let mut any_neg = false;
    let mut any_huge = false;
    let mut any_bool = false;
    for it in &items {
        match it {
            Num::Null => any_null = true,
            Num::NegZero => any_int = true,
            Num::Float(_) => any_float = true,
            Num::Huge(_) => any_huge = true,
            Num::Bool(_) => any_bool = true,
            Num::Int(i) => {
                if *i > i128::from(i64::MAX) {
                    if *i > i128::from(u64::MAX) {
                        any_huge = true;
                    } else {
                        any_uint = true;
                    }
                } else {
                    any_int = true;
                    if *i < 0 {
                        any_neg = true;
                    }
                }
            }
        }
    }
    // check_uint64_conflict: above-int64 values next to negatives or NaN (checked before floats).
    if any_uint && (any_neg || any_null) {
        return Attempt::Raw;
    }
    let as_f64 = |it: &Num| -> f64 {
        match it {
            Num::Null => f64::NAN,
            Num::NegZero => -0.0,
            Num::Int(i) => *i as f64,
            Num::Huge(f) | Num::Float(f) => *f,
            Num::Bool(b) => f64::from(u8::from(*b)),
        }
    };
    if any_float || any_null {
        return Attempt::Done(
            items
                .iter()
                .map(|it| {
                    let f = as_f64(it);
                    if f.is_nan() {
                        String::new()
                    } else {
                        fmt_1f(f)
                    }
                })
                .collect(),
        );
    }
    if any_huge {
        return Attempt::Object;
    }
    if any_int || any_uint {
        return Attempt::Done(
            items
                .iter()
                .map(|it| match it {
                    Num::Int(i) => i.to_string(),
                    Num::NegZero => "0".to_string(),
                    Num::Bool(b) => u8::from(*b).to_string(),
                    _ => String::new(),
                })
                .collect(),
        );
    }
    if any_bool {
        return Attempt::Done(
            items
                .iter()
                .map(|it| match it {
                    Num::Bool(true) => "True".to_string(),
                    _ => "False".to_string(),
                })
                .collect(),
        );
    }
    Attempt::Done(vec![String::new(); items.len()])
}

/// The text of an object column: `sanitize_objects` (NA strings become NaN,
/// equal values collapse onto the first one seen, so `True` after `1` prints
/// `1`), `maybe_convert_bool` (all values bool or `True/TRUE/true/False/...`
/// text, NaN allowed), datetime inference (all values `datetime`), else the
/// `str()` of each value (`float_format` is not applied).
fn object_column(vals: &[Cell]) -> Vec<String> {
    // sanitize_objects: NA strings to NaN; memo of equal values.
    let mut cells: Vec<Cell> = Vec::with_capacity(vals.len());
    let mut memo: HashMap<String, Cell> = HashMap::new();
    for v in vals {
        let c = match v {
            Cell::Blank => Cell::Nan,
            Cell::Str(s) if NA_VALUES.contains(&s.as_str()) => Cell::Nan,
            other => other.clone(),
        };
        // Python equality classes that can collide: 0/False and 1/True.
        let key = match &c {
            Cell::Int(0) | Cell::Bool(false) => Some("n0".to_string()),
            Cell::Int(1) | Cell::Bool(true) => Some("n1".to_string()),
            _ => None,
        };
        match key {
            Some(k) => cells.push(memo.entry(k).or_insert_with(|| c.clone()).clone()),
            None => cells.push(c),
        }
    }
    // maybe_convert_bool (skipped when the first value is an int or bool).
    let first_is_int = matches!(cells.first(), Some(Cell::Int(_) | Cell::Bool(_)));
    if !first_is_int && !cells.is_empty() {
        let bools: Option<Vec<Option<bool>>> = cells
            .iter()
            .map(|c| match c {
                Cell::Nan => Some(None),
                Cell::Bool(b) => Some(Some(*b)),
                Cell::Str(s) if matches!(s.as_str(), "True" | "TRUE" | "true") => Some(Some(true)),
                Cell::Str(s) if matches!(s.as_str(), "False" | "FALSE" | "false") => {
                    Some(Some(false))
                }
                _ => None,
            })
            .collect();
        if let Some(b) = bools {
            return b
                .iter()
                .map(|x| match x {
                    Some(true) => "True".to_string(),
                    Some(false) => "False".to_string(),
                    None => String::new(),
                })
                .collect();
        }
    }
    // DataFrame construction: all datetime (or NaN) becomes datetime64.
    let any_dt = cells.iter().any(|c| matches!(c, Cell::DateTime(_)));
    if any_dt
        && cells
            .iter()
            .all(|c| matches!(c, Cell::DateTime(_) | Cell::Nan))
    {
        let vs: Vec<Option<NaiveDateTime>> = cells
            .iter()
            .map(|c| {
                if let Cell::DateTime(d) = c {
                    Some(*d)
                } else {
                    None
                }
            })
            .collect();
        return format_datetime64(&vs);
    }
    cells.iter().map(cell_str).collect()
}

/// Python `str()` of a cell as it sits in an object column.
fn cell_str(c: &Cell) -> String {
    match c {
        Cell::Blank | Cell::Nan => String::new(),
        Cell::Str(s) => s.clone(),
        Cell::Int(i) => i.to_string(),
        Cell::Huge(t, _) => t.clone(),
        Cell::Float(f) => py_repr_float(*f),
        Cell::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Cell::DateTime(d) => str_datetime(d),
        Cell::Time(t) => str_time(t),
    }
}

/// `TextParser` type inference of one data column, printed as `to_csv` does.
fn column_text(vals: &[Cell]) -> Vec<String> {
    match numeric_attempt(vals) {
        Attempt::Done(col) => col,
        Attempt::Object => object_column(vals),
        Attempt::Raw => vals.iter().map(cell_str).collect(),
    }
}

/// Key used by the header de-duplication: Python equality between a text
/// name and a non-text name never holds.
#[derive(Clone, PartialEq, Eq, Hash)]
struct NameKey(u8, String);

/// A header label with its pandas Index flavour.
#[derive(Clone)]
struct Label {
    key: NameKey,
    kind: LabelKind,
    int: Option<i128>,
    float: Option<f64>,
    dt: Option<NaiveDateTime>,
}

#[derive(Clone, Copy, PartialEq)]
enum LabelKind {
    Str,
    Int,
    Float,
    Other,
    DateTime,
}

fn header_labels(row: &[Cell]) -> Vec<Label> {
    let mk = |kind: LabelKind, tag: u8, text: String| Label {
        key: NameKey(tag, text),
        kind,
        int: None,
        float: None,
        dt: None,
    };
    let mut out = Vec::with_capacity(row.len());
    for (i, c) in row.iter().enumerate() {
        out.push(match c {
            Cell::Blank => mk(LabelKind::Str, 0, format!("Unnamed: {i}")),
            Cell::Str(s) => mk(LabelKind::Str, 0, s.clone()),
            Cell::Int(v) if i64::try_from(*v).is_ok() => Label {
                int: Some(*v),
                ..mk(LabelKind::Int, 1, v.to_string())
            },
            Cell::Int(v) => mk(LabelKind::Other, 1, v.to_string()),
            Cell::Huge(t, _) => mk(LabelKind::Other, 1, t.clone()),
            Cell::Float(f) => Label {
                float: Some(*f),
                ..mk(LabelKind::Float, 1, py_repr_float(*f))
            },
            Cell::Nan => mk(LabelKind::Other, 4, String::new()),
            Cell::Bool(_) => mk(LabelKind::Other, 1, cell_str(c)),
            Cell::DateTime(d) => Label {
                dt: Some(*d),
                ..mk(LabelKind::DateTime, 2, str_datetime(d))
            },
            Cell::Time(_) => mk(LabelKind::Other, 3, cell_str(c)),
        });
    }
    out
}

/// `PythonParser._infer_columns` header mangling: empty cells become
/// `Unnamed: N`, then duplicates get `.1`, `.2` suffixes; named columns are
/// processed before unnamed ones and a generated name that already exists in
/// the header is skipped.
fn dedup_header(labels: &mut [Label], blanks: &[bool]) {
    let mut counts: HashMap<NameKey, usize> = HashMap::new();
    let order: Vec<usize> = (0..labels.len())
        .filter(|i| !blanks[*i])
        .chain((0..labels.len()).filter(|i| blanks[*i]))
        .collect();
    for i in order {
        let mut col = labels[i].clone();
        let old = col.key.clone();
        let mut cur = counts.get(&col.key).copied().unwrap_or(0);
        if cur > 0 {
            while cur > 0 {
                counts.insert(old.clone(), cur + 1);
                let name = format!("{}.{}", old.1, cur);
                col = Label {
                    key: NameKey(0, name),
                    kind: LabelKind::Str,
                    int: None,
                    float: None,
                    dt: None,
                };
                if labels.iter().any(|l| l.key == col.key) {
                    cur += 1;
                } else {
                    cur = counts.get(&col.key).copied().unwrap_or(0);
                }
            }
        }
        counts.insert(col.key.clone(), cur + 1);
        labels[i] = col;
    }
}

/// Header text as `to_csv` writes the column Index: all ints print as ints,
/// ints and floats together become a float Index printed with `%.1f`, all
/// datetimes become a `DatetimeIndex`, anything else is text.
fn header_text(labels: &[Label]) -> Vec<String> {
    let all = |k: &[LabelKind]| labels.iter().all(|l| k.contains(&l.kind));
    if all(&[LabelKind::Int, LabelKind::Float]) && labels.iter().any(|l| l.kind == LabelKind::Float)
    {
        return labels
            .iter()
            .map(|l| fmt_1f(l.float.unwrap_or_else(|| l.int.unwrap_or(0) as f64)))
            .collect();
    }
    if all(&[LabelKind::DateTime]) && !labels.is_empty() {
        let vs: Vec<Option<NaiveDateTime>> = labels.iter().map(|l| l.dt).collect();
        return format_datetime64(&vs);
    }
    labels.iter().map(|l| l.key.1.clone()).collect()
}

/// Append one CSV field as Python's `csv.writer` (QUOTE_MINIMAL) does.
fn push_field(out: &mut String, field: &str, lone_field: bool) {
    if field.is_empty() && lone_field {
        out.push_str("\"\"");
    } else if field.contains([',', '"', '\n', '\r']) {
        out.push('"');
        out.push_str(&field.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(field);
    }
}

/// Port of `read_excel(...)` then `to_csv(index=False, float_format="%.1f")`.
///
/// `data` is pandas' list-of-lists (rows of equal width, header row first,
/// no rows at all for an empty sheet). Confirmed rules:
///
/// * the first row is the header; empty header cells become `Unnamed: N`
///   (N = column position), duplicates `X`, `X.1`, `X.2` (see
///   [`dedup_header`]); numeric header labels print per Index dtype;
/// * an empty sheet gives `"\n"`; a sheet with only a header row gives the
///   header line;
/// * every data column is inferred like `maybe_convert_numeric`: all-integer
///   columns print `4`; blanks/NA strings/floats make a float64 column
///   printed with `%.1f` (`4.0`); numbers stored as text convert too (`"007"`
///   prints `7`); `True/False` text becomes booleans;
/// * columns that stay `object` print each value's own `str()` (no
///   `float_format`); NA strings and blanks print empty;
/// * datetimes form a datetime64 column (date only when all at midnight);
/// * CSV quoting is `QUOTE_MINIMAL` and a row whose only field is empty is
///   written as `""`.
pub fn sheet_to_csv_text(data: &[Vec<Cell>]) -> String {
    if data.is_empty() {
        return "\n".to_string();
    }
    let ncols = data[0].len();
    let blanks: Vec<bool> = data[0].iter().map(|c| matches!(c, Cell::Blank)).collect();
    let mut labels = header_labels(&data[0]);
    dedup_header(&mut labels, &blanks);
    let header = header_text(&labels);

    let nrows = data.len() - 1;
    let columns: Vec<Vec<String>> = (0..ncols)
        .map(|c| {
            let vals: Vec<Cell> = data[1..]
                .iter()
                .map(|r| r.get(c).cloned().unwrap_or(Cell::Blank))
                .collect();
            column_text(&vals)
        })
        .collect();

    let mut out = String::new();
    for (i, h) in header.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_field(&mut out, h, ncols == 1);
    }
    out.push('\n');
    for r in 0..nrows {
        for (c, col) in columns.iter().enumerate() {
            if c > 0 {
                out.push(',');
            }
            push_field(&mut out, &col[r], ncols == 1);
        }
        out.push('\n');
    }
    out
}

/// openpyxl `from_excel` (also used for `.xls`, unverified there): an Excel
/// serial number to a `datetime` or, for values in `[0, 1)`, a `time`.
pub fn excel_serial_to_cell(value: f64, epoch_1904: bool) -> Cell {
    let day = value.floor();
    let fraction = value - day;
    let ms = (fraction * 86400.0 * 1000.0).round_ties_even();
    if !ms.is_finite() || !day.is_finite() || day.abs() > 3.0e6 {
        return Cell::Float(value);
    }
    let ms = ms as i64;
    let diff_days = ms.div_euclid(86_400_000);
    if (0.0..1.0).contains(&value) && diff_days == 0 {
        let secs = (ms / 1000) as u32;
        let micro = ((ms % 1000) * 1000) as u32;
        return match NaiveTime::from_hms_micro_opt(secs / 3600, (secs / 60) % 60, secs % 60, micro)
        {
            Some(t) => Cell::Time(t),
            None => Cell::Float(value),
        };
    }
    let mut day = day as i64;
    if value > 0.0 && value < 60.0 && !epoch_1904 {
        day += 1;
    }
    let epoch = if epoch_1904 {
        NaiveDate::from_ymd_opt(1904, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(1899, 12, 30)
    };
    let dt = epoch
        .and_then(|e| e.and_hms_opt(0, 0, 0))
        .and_then(|e| e.checked_add_signed(chrono::Duration::days(day)))
        .and_then(|e| e.checked_add_signed(chrono::Duration::milliseconds(ms)));
    match dt {
        Some(d) => Cell::DateTime(d),
        None => Cell::Float(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_float_matches_python() {
        assert_eq!(py_repr_float(3.8428571428571425), "3.8428571428571425");
        assert_eq!(py_repr_float(1e16), "1e+16");
        assert_eq!(py_repr_float(1.5e-5), "1.5e-05");
        assert_eq!(py_repr_float(100.0), "100.0");
        assert_eq!(py_repr_float(0.0001), "0.0001");
        assert_eq!(
            py_repr_float(123456789012345680.0),
            "1.2345678901234568e+17"
        );
        assert_eq!(py_repr_float(-2.5), "-2.5");
    }

    #[test]
    fn floatify_grammar() {
        assert_eq!(floatify(" 5 "), Some((5.0, true)));
        assert_eq!(floatify("1e3"), Some((1000.0, false)));
        assert!(floatify("1e").is_none());
        assert!(floatify("1_000").is_none());
        assert!(floatify("NAN").is_none());
        assert!(floatify("-Infinity").is_some());
        assert!(floatify("\u{a0}1").is_none());
    }
}
