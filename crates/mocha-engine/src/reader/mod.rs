//! Spreadsheet reading: produces the checkpoint-A grid, the string grid the
//! Python parser sees after `pd.read_excel` -> `to_csv(float_format="%.1f")`
//! -> `pd.read_csv(header=None, dtype=str, keep_default_na=False)`.

pub mod csv;
pub mod normalize;
pub mod sheets;
pub mod workbook;

use std::path::Path;

use crate::error::ReadError;
use crate::model::Grid;

pub use sheets::get_sheet_names;

/// Result of [`read_grid`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOutput {
    /// Checkpoint-A grid.
    pub grid: Grid,
    /// Name of the intermediate CSV as Python has it (Excel with sheet:
    /// `<source stem>_<sheet>.csv`; Excel without sheet: the source path with
    /// a `.csv` suffix; CSV source: the source path as given).
    pub csv_file_name: String,
}

/// Python `Path.suffix` of a file name: text from the last dot, unless the
/// only dot is the first character or the name ends with a dot.
fn py_suffix(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => &name[i..],
        _ => "",
    }
}

/// Python `Path.stem`.
fn py_stem(name: &str) -> &str {
    let s = py_suffix(name);
    &name[..name.len() - s.len()]
}

fn file_name(source: &Path) -> String {
    source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Mirrors `GradebookProcessor.convert()` (py lines 1717-1780) plus the
/// parser's read (py line 665). With a sheet name on an Excel file only that
/// sheet is read (no audit); otherwise `FileConverter.convert_and_audit`
/// (py lines 323-415) runs: Excel is converted from its first sheet, CSV is
/// audited, and a status other than success aborts. Nothing is written to
/// disk.
///
/// # Errors
///
/// A [`ReadError`] whose [`ReadError::status`] is the Python audit status.
pub fn read_grid(source: &Path, sheet: Option<&str>) -> Result<ReadOutput, ReadError> {
    let name = file_name(source);
    let lower = source.to_string_lossy().to_lowercase();
    let is_excel_name = lower.ends_with(".xls") || lower.ends_with(".xlsx");
    let to_grid_err = |message: String| ReadError::GridRead { message };

    if let Some(sheet) = sheet.filter(|s| !s.is_empty() && is_excel_name) {
        // py 1721-1759: sheet-based processing, no audit.
        let mut book = workbook::open(source, "failed")?;
        let data = book.sheet_data(sheet)?;
        let text = normalize::sheet_to_csv_text(&data);
        let grid = csv::read_final(&text).map_err(to_grid_err)?;
        return Ok(ReadOutput {
            grid,
            csv_file_name: format!("{}_{}.csv", py_stem(&name), sheet),
        });
    }

    // py 1762-1780: convert_and_audit.
    let suffix = py_suffix(&name).to_lowercase();
    let (text, csv_file_name) = match suffix.as_str() {
        ".xls" | ".xlsx" => {
            let mut book = workbook::open(source, "conversion_failed")?;
            let first =
                book.sheet_names()
                    .into_iter()
                    .next()
                    .ok_or_else(|| ReadError::Corrupted {
                        message: "At least one sheet must be visible".to_string(),
                        status: "conversion_failed",
                    })?;
            let data = book.sheet_data(&first).map_err(|e| ReadError::Corrupted {
                message: e.to_string(),
                status: "conversion_failed",
            })?;
            let out = source.with_extension("csv").to_string_lossy().into_owned();
            (normalize::sheet_to_csv_text(&data), out)
        }
        ".csv" => {
            let bytes =
                std::fs::read(source).map_err(|e| workbook::io_error(source, &e, "corrupted"))?;
            let text = String::from_utf8(bytes).map_err(|e| ReadError::Corrupted {
                message: format!("'utf-8' codec can't decode byte: {e}"),
                status: "corrupted",
            })?;
            (text, source.to_string_lossy().into_owned())
        }
        other => {
            return Err(ReadError::UnsupportedFormat {
                extension: other.to_string(),
            });
        }
    };
    match csv::audit(&text) {
        csv::Audit::Ok => {}
        csv::Audit::Empty => return Err(ReadError::Empty),
        csv::Audit::Failed(message) => {
            return Err(ReadError::Corrupted {
                message,
                status: "corrupted",
            });
        }
    }
    let grid = csv::read_final(&text).map_err(to_grid_err)?;
    Ok(ReadOutput {
        grid,
        csv_file_name,
    })
}

/// Run only the CSV-text step for an Excel sheet (Excel to `to_csv` text),
/// used by tests to compare with the oracle's `intermediate.csv`.
///
/// # Errors
///
/// A [`ReadError`] when the workbook or sheet cannot be read.
pub fn excel_sheet_csv_text(source: &Path, sheet: &str) -> Result<String, ReadError> {
    let mut book = workbook::open(source, "failed")?;
    let data = book.sheet_data(sheet)?;
    Ok(normalize::sheet_to_csv_text(&data))
}
