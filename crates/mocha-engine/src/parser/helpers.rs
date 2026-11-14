//! Grid-access helpers shared by the tabular and carnet parsers: the
//! `GradebookParser` methods that only look at the raw grid
//! (`_find_header_col`, `_find_header_prefix_col`, `_find_name_col`,
//! `_first_non_empty_in_row`, `_first_matching_in_row`, `_row_is_blank`,
//! `_first_numeric_header_col`) plus small Python-semantics utilities.

use regex::Regex;

use crate::model::Grid;
use crate::pycompat::DECIMAL_ZERO_STARTS;
use crate::text::{clean_text, normalize_header_token, to_float};

use super::ParseError;

/// Header names of the student-number column (py lines 677, 867, 884, 1039).
pub(crate) const NO_NAMES: &[&str] = &["no", "no.", "n\u{b0}", "n\u{ba}"];

/// The cells of grid row `row_idx` (`raw_df.iloc[row_idx].tolist()`); an
/// out-of-range row gives an empty slice.
pub(crate) fn row_cells(grid: &Grid, row_idx: usize) -> &[String] {
    grid.get(row_idx).map_or(&[], Vec::as_slice)
}

/// `[clean_text(v) for v in row]` (py lines 674, 675, 883).
pub(crate) fn clean_row(cells: &[String]) -> Vec<String> {
    cells.iter().map(|v| clean_text(v)).collect()
}

/// `[clean_text(v).lower() for v in row]` (py lines 865, 1038).
pub(crate) fn clean_lower_row(cells: &[String]) -> Vec<String> {
    cells.iter().map(|v| clean_text(v).to_lowercase()).collect()
}

/// Port of `_find_header_col` (py 1104-1109): first index whose normalized
/// token is one of the normalized `names`; `default` otherwise.
pub(crate) fn find_header_col(
    header: &[String],
    names: &[&str],
    default: Option<usize>,
) -> Option<usize> {
    let normalized_names: Vec<String> = names.iter().map(|n| normalize_header_token(n)).collect();
    header
        .iter()
        .position(|token| normalized_names.contains(&normalize_header_token(token)))
        .or(default)
}

/// Port of `_find_header_prefix_col` (py 1112-1122): first index whose
/// normalized token starts with one of `prefixes` (taken as given, like the
/// Python literals); `default` otherwise.
pub(crate) fn find_header_prefix_col(
    header: &[String],
    prefixes: &[&str],
    default: Option<usize>,
) -> Option<usize> {
    header
        .iter()
        .position(|token| {
            let normalized = normalize_header_token(token);
            prefixes.iter().any(|p| normalized.starts_with(p))
        })
        .or(default)
}

/// Port of `_find_name_col` (py 1125-1137): first index whose normalized
/// token contains a name-like word; `default` otherwise.
pub(crate) fn find_name_col(header: &[String], default: Option<usize>) -> Option<usize> {
    header
        .iter()
        .position(|token| {
            let normalized = normalize_header_token(token);
            [
                "nom",
                "pr\u{e9}nom",
                "prenom",
                "\u{e9}tudiant",
                "etudiant",
                "estudiante",
            ]
            .iter()
            .any(|w| normalized.contains(w))
        })
        .or(default)
}

/// Port of `_first_non_empty_in_row` (py 1139-1146): first non-empty cleaned
/// cell of the row; `""` when the row is beyond the grid or all blank.
pub(crate) fn first_non_empty_in_row(grid: &Grid, row_idx: usize) -> String {
    if row_idx >= grid.len() {
        return String::new();
    }
    for value in row_cells(grid, row_idx) {
        let text = clean_text(value);
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
}

/// Port of `_first_matching_in_row` (py 1148-1156): first cleaned cell the
/// (ignore-case) regex finds a match in, else [`first_non_empty_in_row`]. A
/// `None` regex (uncompilable pattern) never matches.
pub(crate) fn first_matching_in_row(grid: &Grid, row_idx: usize, regex: Option<&Regex>) -> String {
    if row_idx >= grid.len() {
        return String::new();
    }
    for value in row_cells(grid, row_idx) {
        let text = clean_text(value);
        if !text.is_empty() && regex.is_some_and(|r| r.is_match(&text)) {
            return text;
        }
    }
    first_non_empty_in_row(grid, row_idx)
}

/// Port of `_row_is_blank` (py 1090-1091): every cell cleans to `""`.
pub(crate) fn row_is_blank(cells: &[String]) -> bool {
    cells.iter().all(|v| clean_text(v).is_empty())
}

/// Port of `_first_numeric_header_col` (py 1067-1072): first index after
/// `start_after` whose cleaned text parses as a float.
pub(crate) fn first_numeric_header_col(
    header: &[String],
    start_after: Option<usize>,
) -> Option<usize> {
    let start = start_after.map_or(0, |s| s + 1);
    (start..header.len()).find(|&idx| to_float(&header[idx]).is_some())
}

/// Decimal value of a Unicode `Nd` digit (what Python `int()` accepts).
fn decimal_digit(c: char) -> Option<u8> {
    let cp = c as u32;
    let idx = DECIMAL_ZERO_STARTS.partition_point(|&s| s <= cp);
    let start = *DECIMAL_ZERO_STARTS.get(idx.checked_sub(1)?)?;
    u8::try_from(cp - start).ok().filter(|&d| d < 10)
}

/// Python `str(int(digits))` for a text that matched `\d+`: Unicode digits
/// become ASCII, leading zeros vanish. Python 3.11+ refuses strings of more
/// than 4300 digits (`ValueError`), which maps to
/// [`ParseError::IntegerTooLarge`].
pub(crate) fn py_int_str(digits: &str) -> Result<String, ParseError> {
    let count = digits.chars().count();
    if count > 4300 {
        return Err(ParseError::IntegerTooLarge { digits: count });
    }
    let ascii: String = digits
        .chars()
        .map(|c| decimal_digit(c).map_or(c, |d| char::from(b'0' + d)))
        .collect();
    let trimmed = ascii.trim_start_matches('0');
    Ok(if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    })
}

/// Strict `row[col]` (pandas label access): a missing column is a `KeyError`
/// in Python, [`ParseError::IndexOutOfRange`] here.
pub(crate) fn cell(row: &[String], col: usize) -> Result<&str, ParseError> {
    row.get(col)
        .map(String::as_str)
        .ok_or(ParseError::IndexOutOfRange {
            index: col,
            len: row.len(),
        })
}

/// Lenient `row[col] if col in row.index else ""` as used by `row_value` and
/// the `... in row.index` guards.
pub(crate) fn cell_or_empty(row: &[String], col: usize) -> &str {
    row.get(col).map_or("", String::as_str)
}
