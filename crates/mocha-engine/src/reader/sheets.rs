//! Port of `GradebookProcessor.get_sheet_names` (py lines 1648-1673) and
//! `_sheet_has_gradebook_header` (py lines 1675-1715).

use std::path::Path;

use super::normalize::Cell;
use super::workbook::{open, Book};
use crate::text::clean_text;

/// Sheet names of an Excel file; by default only gradebook-like sheets.
///
/// Returns an empty list for non-Excel extensions and for any read failure,
/// as the Python `except Exception` does.
pub fn get_sheet_names(source: &Path, include_empty: bool) -> Vec<String> {
    let lower = source.to_string_lossy().to_lowercase();
    if !(lower.ends_with(".xls") || lower.ends_with(".xlsx")) {
        return Vec::new();
    }
    let Ok(mut book) = open(source, "failed") else {
        return Vec::new();
    };
    let names = book.sheet_names();
    if include_empty {
        return names;
    }
    names
        .into_iter()
        .filter(|n| sheet_has_gradebook_header(&mut book, n))
        .collect()
}

/// Port of `_sheet_has_gradebook_header`: the first 15 rows (counted from
/// the sheet's first row) are searched for a header row.
fn sheet_has_gradebook_header(book: &mut Book, sheet_name: &str) -> bool {
    let Ok(data) = book.sheet_data(sheet_name) else {
        return false;
    };
    for row in data.iter().take(15) {
        let tokens: Vec<String> = row
            .iter()
            .map(|c| match c {
                Cell::Str(s) => clean_text(s).to_lowercase(),
                _ => String::new(),
            })
            .collect();
        let has = |t: &str| tokens.iter().any(|x| x == t);
        let has_number = tokens
            .iter()
            .any(|t| matches!(t.as_str(), "no" | "no." | "n°" | "nº"));
        let has_code = has("code");
        let has_student_name = tokens.iter().any(|t| {
            t.contains("nom")
                || t.contains("prénom")
                || t.contains("prenom")
                || t.contains("étudiant")
                || t.contains("etudiant")
                || t.contains("estudiante")
        });
        let has_absences = tokens.iter().any(|t| t.starts_with("abse"));
        if has_number && has_code && has_student_name && has_absences {
            return true;
        }
        if has_number && has("item") && has("note") && has("moy") {
            return true;
        }
    }
    false
}
