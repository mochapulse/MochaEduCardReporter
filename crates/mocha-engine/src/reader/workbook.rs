//! `.xls` / `.xlsx` access through calamine, producing the list-of-lists that
//! pandas' Excel readers (`openpyxl` for `.xlsx`, `xlrd` for `.xls`) pass to
//! its text parser.

use std::io::Cursor;
use std::path::Path;

use calamine::{Data, Reader, Xls, Xlsx};

use super::normalize::{excel_serial_to_cell, Cell};
use crate::error::ReadError;

/// An open workbook (format chosen by file content, as pandas does).
pub(crate) enum Book {
    Xlsx(Xlsx<Cursor<Vec<u8>>>),
    Xls(Xls<Cursor<Vec<u8>>>),
}

/// Map an I/O error to a [`ReadError`] carrying `status`.
pub(crate) fn io_error(path: &Path, err: &std::io::Error, status: &'static str) -> ReadError {
    let p = path.to_string_lossy().into_owned();
    match err.kind() {
        std::io::ErrorKind::NotFound => ReadError::NotFound { path: p, status },
        std::io::ErrorKind::PermissionDenied => ReadError::Permission { path: p, status },
        _ => ReadError::Corrupted {
            message: err.to_string(),
            status,
        },
    }
}

/// Open a workbook. `status` is the Python audit status used for failures.
pub(crate) fn open(path: &Path, status: &'static str) -> Result<Book, ReadError> {
    let bytes = std::fs::read(path).map_err(|e| io_error(path, &e, status))?;
    let corrupted = |message: String| ReadError::Corrupted { message, status };
    if bytes.starts_with(b"PK") {
        Xlsx::new(Cursor::new(bytes))
            .map(Book::Xlsx)
            .map_err(|e| corrupted(e.to_string()))
    } else if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        Xls::new(Cursor::new(bytes))
            .map(Book::Xls)
            .map_err(|e| corrupted(e.to_string()))
    } else {
        Err(corrupted(
            "Excel file format cannot be determined, you must specify an engine manually."
                .to_string(),
        ))
    }
}

/// Convert a calamine value like pandas' reader callbacks do: integral floats
/// become `int`, error cells `NaN`, empty text blank.
fn convert(data: &Data, epoch_1904: bool) -> Cell {
    match data {
        Data::Empty => Cell::Blank,
        Data::String(s) if s.is_empty() => Cell::Blank,
        Data::String(s) => Cell::Str(s.clone()),
        Data::Int(i) => Cell::Int(i128::from(*i)),
        Data::Float(f) => float_cell(*f),
        Data::Bool(b) => Cell::Bool(*b),
        Data::DateTime(d) => excel_serial_to_cell(d.as_f64(), epoch_1904),
        Data::DateTimeIso(s) | Data::DurationIso(s) => Cell::Str(s.clone()),
        Data::Error(_) => Cell::Nan,
    }
}

fn float_cell(f: f64) -> Cell {
    if f.is_finite() && f.fract() == 0.0 {
        if f.abs() < 1.0e38 {
            Cell::Int(f as i128)
        } else {
            Cell::Huge(format!("{f:.0}"), f)
        }
    } else {
        Cell::Float(f)
    }
}

impl Book {
    /// Sheet names in workbook order (hidden sheets included).
    pub(crate) fn sheet_names(&self) -> Vec<String> {
        match self {
            Book::Xlsx(b) => b.sheet_names(),
            Book::Xls(b) => b.sheet_names(),
        }
    }

    /// pandas' list-of-lists for one sheet (header row first). Rows start at
    /// the sheet's first row and column (A1) even when calamine's used range
    /// starts later. `.xlsx` (openpyxl) trims trailing empty cells and rows;
    /// `.xls` (xlrd) does not. Rows are padded to a common width.
    pub(crate) fn sheet_data(&mut self, name: &str) -> Result<Vec<Vec<Cell>>, ReadError> {
        let (range, is_xls, epoch_1904) = match self {
            Book::Xlsx(b) => (
                b.worksheet_range(name).map_err(|e| e.to_string()),
                false,
                false,
            ),
            Book::Xls(b) => {
                let e = b.has_1904_epoch();
                (b.worksheet_range(name).map_err(|e| e.to_string()), true, e)
            }
        };
        let range = match range {
            Ok(r) => r,
            Err(msg) => {
                if self.sheet_names().iter().any(|n| n == name) {
                    return Err(ReadError::Corrupted {
                        message: msg,
                        status: "failed",
                    });
                }
                return Err(ReadError::SheetNotFound {
                    sheet: name.to_string(),
                });
            }
        };
        let Some((r0, c0)) = range.start() else {
            return Ok(Vec::new());
        };
        let (r0, c0) = (r0 as usize, c0 as usize);
        let mut data: Vec<Vec<Cell>> = Vec::with_capacity(r0 + range.height());
        for _ in 0..r0 {
            data.push(Vec::new());
        }
        for row in range.rows() {
            let mut cells: Vec<Cell> = vec![Cell::Blank; c0];
            cells.extend(row.iter().map(|d| convert(d, epoch_1904)));
            data.push(cells);
        }
        if !is_xls {
            for r in &mut data {
                while matches!(r.last(), Some(Cell::Blank)) {
                    r.pop();
                }
            }
            while data.last().is_some_and(Vec::is_empty) {
                data.pop();
            }
        }
        let width = data.iter().map(Vec::len).max().unwrap_or(0);
        for r in &mut data {
            r.resize(width, Cell::Blank);
        }
        if width == 0 {
            data.clear();
        }
        Ok(data)
    }
}
