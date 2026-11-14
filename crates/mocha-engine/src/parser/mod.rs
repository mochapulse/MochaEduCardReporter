//! Gradebook parser: port of `GradebookParser` (py lines 554-1176).
//!
//! Input is the checkpoint-A grid; output is checkpoint B (metadata and
//! students) plus the WARNING-level log messages the Python parser emits.

mod carnet;
mod helpers;
pub mod labels;
mod tabular;

use thiserror::Error;

use crate::model::{FileMetadata, Grid, StudentGrades};

/// Result of [`parse`]: checkpoint B plus the parser's warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    /// Extracted file metadata.
    pub metadata: FileMetadata,
    /// Students, in sheet order.
    pub students: Vec<StudentGrades>,
    /// Messages Python logs at WARNING level, in order.
    pub warnings: Vec<String>,
}

/// Every place where the Python parser raises on its input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// Python `ValueError("Could not find a supported gradebook or carnet layout")`.
    #[error("Could not find a supported gradebook or carnet layout")]
    NoSupportedLayout,
    /// `int(float(token))` raised `OverflowError` on a header number so large
    /// that `float()` gives `inf`.
    #[error("grade number {cell:?} is too large to convert")]
    InvalidGradeNumber {
        /// The offending header cell.
        cell: String,
    },
    /// `int(text)` raised `ValueError` ("Exceeds the limit (4300 digits)").
    #[error("integer with {digits} digits exceeds the 4300 digit limit")]
    IntegerTooLarge {
        /// Number of digits of the offending cell.
        digits: usize,
    },
    /// Python `IndexError`/`KeyError` while reading a cell outside the row.
    #[error("column {index} is out of range for a row of {len} cells")]
    IndexOutOfRange {
        /// Requested column.
        index: usize,
        /// Row length.
        len: usize,
    },
}

/// Mirrors `GradebookParser.parse(csv_file)` (py 648-860) given the
/// already-read grid. `csv_file` is the path/name Python would have received:
/// its file name becomes `metadata.file_name`, and the carnet layout derives
/// the period and group label from its stem.
pub fn parse(grid: &Grid, csv_file: &str) -> Result<ParseOutput, ParseError> {
    match tabular::find_header_row(grid) {
        Some(header_row_idx) => tabular::parse_tabular(grid, csv_file, header_row_idx),
        None => carnet::parse_carnet_blocks(grid, csv_file),
    }
}
