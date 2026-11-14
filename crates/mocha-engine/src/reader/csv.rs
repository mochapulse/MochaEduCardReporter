//! Emulation of `pd.read_csv` (C engine) on the CSV text the engine reads:
//! the audit pre-read `read_csv(handle)` (py line 378) and the parser's read
//! `read_csv(handle, header=None, dtype=str, keep_default_na=False)`
//! (py line 665).
//!
//! The Rust `csv` crate is deliberately not used: its quoting and ragged-row
//! rules differ from pandas' tokenizer (confirmed against pandas 3.0.1).

use crate::model::Grid;

/// Result of the audit pre-read (`FileConverter.convert_and_audit`).
#[derive(Debug, PartialEq, Eq)]
pub enum Audit {
    /// Header and at least one data row.
    Ok,
    /// Header only, `df.empty` is true (status `empty`).
    Empty,
    /// `read_csv` raised (status `corrupted`); the pandas-style message.
    Failed(String),
}

/// pandas' C tokenizer: records of fields, or the tokenizer error text.
///
/// Confirmed rules: one leading BOM is dropped; `\n`, `\r\n` and a lone `\r`
/// end a record (inside quotes they are kept verbatim); lines that are empty
/// or only spaces/tabs are skipped; a quote starts a quoted field only at the
/// start of a field (elsewhere it is literal); `""` inside quotes is a quote;
/// text after a closing quote is appended to the field; a field is cut at its
/// first NUL; EOF inside quotes is an error; a missing final newline is fine.
fn tokenize(text: &str) -> Result<Vec<Vec<String>>, String> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = usize::from(chars.first() == Some(&'\u{feff}'));
    let mut rows: Vec<Vec<String>> = Vec::new();
    while i < n {
        // Blank or whitespace-only line.
        let mut j = i;
        while j < n && (chars[j] == ' ' || chars[j] == '\t') {
            j += 1;
        }
        if j >= n {
            break;
        }
        if chars[j] == '\n' {
            i = j + 1;
            continue;
        }
        if chars[j] == '\r' {
            i = j + 1;
            if i < n && chars[i] == '\n' {
                i += 1;
            }
            continue;
        }
        let mut fields: Vec<String> = Vec::new();
        'record: loop {
            let mut field = String::new();
            if i < n && chars[i] == '"' {
                let start_row = rows.len();
                i += 1;
                loop {
                    if i >= n {
                        return Err(format!(
                            "Error tokenizing data. C error: EOF inside string starting at row {start_row}"
                        ));
                    }
                    let ch = chars[i];
                    if ch == '"' {
                        if i + 1 < n && chars[i + 1] == '"' {
                            field.push('"');
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    field.push(ch);
                    i += 1;
                }
                // QUOTE_IN_QUOTED_FIELD: text after the closing quote.
                while i < n && !matches!(chars[i], ',' | '\n' | '\r') {
                    field.push(chars[i]);
                    i += 1;
                }
            } else {
                while i < n && !matches!(chars[i], ',' | '\n' | '\r') {
                    field.push(chars[i]);
                    i += 1;
                }
            }
            if let Some(p) = field.find('\0') {
                field.truncate(p);
            }
            fields.push(field);
            if i >= n {
                break 'record;
            }
            match chars[i] {
                ',' => i += 1,
                '\n' => {
                    i += 1;
                    break 'record;
                }
                _ => {
                    i += 1;
                    if i < n && chars[i] == '\n' {
                        i += 1;
                    }
                    break 'record;
                }
            }
        }
        rows.push(fields);
    }
    Ok(rows)
}

/// Audit pre-read `pd.read_csv(handle)` (header on the first line): which
/// inputs raise and which give an empty frame.
///
/// Confirmed (and fuzzed against pandas): no non-blank line gives
/// "No columns to parse from file"; the first data line may be longer than the
/// header; afterwards a line longer than `max(header, first data line)` is an
/// error; header only (or only blank lines after it) is empty.
pub fn audit(text: &str) -> Audit {
    let rows = match tokenize(text) {
        Ok(r) => r,
        Err(e) => return Audit::Failed(e),
    };
    let Some(header) = rows.first() else {
        return Audit::Failed("No columns to parse from file".to_string());
    };
    let mut expected = header.len();
    if let Some(first) = rows.get(1) {
        expected = expected.max(first.len());
    }
    for (k, r) in rows.iter().enumerate().skip(2) {
        if r.len() > expected {
            return Audit::Failed(format!(
                "Error tokenizing data. C error: Expected {expected} fields in line {}, saw {}\n",
                k + 1,
                r.len()
            ));
        }
    }
    if rows.len() < 2 {
        Audit::Empty
    } else {
        Audit::Ok
    }
}

/// `pd.read_csv(handle, header=None, dtype=str, keep_default_na=False)`
/// followed by `str(v)` for every value: the checkpoint-A grid.
///
/// The width is the first line's; a longer later line is an error, shorter
/// lines are padded with empty strings.
pub fn read_final(text: &str) -> Result<Grid, String> {
    let rows = tokenize(text)?;
    let Some(first) = rows.first() else {
        return Err("No columns to parse from file".to_string());
    };
    let width = first.len();
    let mut grid: Grid = Vec::with_capacity(rows.len());
    for (k, mut r) in rows.into_iter().enumerate() {
        if r.len() > width {
            return Err(format!(
                "Error tokenizing data. C error: Expected {width} fields in line {}, saw {}\n",
                k + 1,
                r.len()
            ));
        }
        r.resize(width, String::new());
        grid.push(r);
    }
    Ok(grid)
}
