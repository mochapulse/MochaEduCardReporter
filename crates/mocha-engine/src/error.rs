//! Errors of the spreadsheet reader.
//!
//! Each variant maps onto the `status` string the Python engine would report
//! in its audit report (`FileConverter.convert_and_audit`, py lines 323-415,
//! and `GradebookProcessor.convert`, py lines 1717-1780), see
//! [`ReadError::status`].

use thiserror::Error;

/// Why a gradebook source could not be turned into a checkpoint-A grid.
#[derive(Debug, Error)]
pub enum ReadError {
    /// The file does not exist.
    #[error("file not found: {path}")]
    NotFound {
        /// Path as given by the caller.
        path: String,
        /// Python audit status for this code path.
        status: &'static str,
    },
    /// The file exists but cannot be opened (locked, permission denied).
    #[error("file locked or permission denied: {path}")]
    Permission {
        /// Path as given by the caller.
        path: String,
        /// Python audit status for this code path.
        status: &'static str,
    },
    /// The extension is neither `.xls`, `.xlsx` nor `.csv` (py 396-400).
    #[error("Unsupported file format: {extension}")]
    UnsupportedFormat {
        /// Extension as Python's `Path.suffix` reports it (with the dot).
        extension: String,
    },
    /// The requested sheet is not in the workbook (pandas `ValueError`).
    #[error("Worksheet named '{sheet}' not found")]
    SheetNotFound {
        /// Requested sheet name.
        sheet: String,
    },
    /// The file could not be read or decoded (Excel format error, invalid
    /// UTF-8, CSV tokenizing error, "No columns to parse from file").
    #[error("{message}")]
    Corrupted {
        /// Message in the spirit of the Python exception text.
        message: String,
        /// Python audit status for this code path.
        status: &'static str,
    },
    /// The CSV audit found a header but no data rows (py 392-393).
    #[error("DataFrame is empty - contains no data rows")]
    Empty,
    /// The audit passed (or was skipped) but the parser's own
    /// `read_csv(header=None, dtype=str, keep_default_na=False)` failed.
    #[error("{message}")]
    GridRead {
        /// Message in the spirit of the pandas exception text.
        message: String,
    },
}

impl ReadError {
    /// The Python `audit_report["status"]` (or the oracle's `parse_error`
    /// for a failure of the parser's own read): `corrupted`, `empty`,
    /// `unsupported_format`, `conversion_failed`, `failed`, `parse_error`.
    pub fn status(&self) -> &'static str {
        match self {
            ReadError::NotFound { status, .. }
            | ReadError::Permission { status, .. }
            | ReadError::Corrupted { status, .. } => status,
            ReadError::UnsupportedFormat { .. } => "unsupported_format",
            ReadError::SheetNotFound { .. } => "failed",
            ReadError::Empty => "empty",
            ReadError::GridRead { .. } => "parse_error",
        }
    }
}

/// Stable, machine-readable error codes (plan section 8). The shell maps each
/// one to a Spanish message with a suggested action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// The source file does not exist.
    FileNotFound,
    /// The source file cannot be opened (locked or permission denied).
    FileLocked,
    /// The extension is not `.xls`, `.xlsx` or `.csv`, or the file content is
    /// not a valid file of that format.
    UnsupportedFormat,
    /// The file or sheet holds no recognizable gradebook, or there is nothing
    /// to generate.
    NoGradebookFound,
    /// The file or sheet is empty.
    EmptySheet,
    /// The output folder cannot be created or written.
    OutputNotWritable,
    /// A PDF could not be laid out.
    PdfFailed,
    /// The user cancelled the operation.
    Cancelled,
    /// Anything else (a bug or an unexpected condition).
    Internal,
}

impl ErrorCode {
    /// The code as the string the UI and CLI use (`FILE_NOT_FOUND`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::FileNotFound => "FILE_NOT_FOUND",
            ErrorCode::FileLocked => "FILE_LOCKED",
            ErrorCode::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            ErrorCode::NoGradebookFound => "NO_GRADEBOOK_FOUND",
            ErrorCode::EmptySheet => "EMPTY_SHEET",
            ErrorCode::OutputNotWritable => "OUTPUT_NOT_WRITABLE",
            ErrorCode::PdfFailed => "PDF_FAILED",
            ErrorCode::Cancelled => "CANCELLED",
            ErrorCode::Internal => "INTERNAL",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Top-level error of the engine's orchestration layer.
///
/// [`EngineError::code`] maps every variant to an [`ErrorCode`]:
///
/// | Error                                              | Code                  |
/// | -------------------------------------------------- | --------------------- |
/// | `ReadError::NotFound`                              | `FILE_NOT_FOUND`      |
/// | `ReadError::Permission`                            | `FILE_LOCKED`         |
/// | `ReadError::UnsupportedFormat`                     | `UNSUPPORTED_FORMAT`  |
/// | `ReadError::Corrupted` ("No columns to parse")     | `EMPTY_SHEET`         |
/// | `ReadError::Corrupted` (any other)                 | `UNSUPPORTED_FORMAT`  |
/// | `ReadError::GridRead`                              | `UNSUPPORTED_FORMAT`  |
/// | `ReadError::Empty`                                 | `EMPTY_SHEET`         |
/// | `ReadError::SheetNotFound`                         | `NO_GRADEBOOK_FOUND`  |
/// | any `ParseError`                                   | `NO_GRADEBOOK_FOUND`  |
/// | `PdfError::Io`                                     | `OUTPUT_NOT_WRITABLE` |
/// | other `PdfError`                                   | `PDF_FAILED`          |
/// | `EngineError::NoStudents` (Python: "Must call parse() first") | `NO_GRADEBOOK_FOUND` |
/// | `EngineError::OutputNotWritable`                   | `OUTPUT_NOT_WRITABLE` |
/// | `EngineError::StudentNotFound`                     | `INTERNAL`            |
/// | `EngineError::Cancelled`                           | `CANCELLED`           |
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Reading the source failed.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The grid has no supported gradebook layout or is malformed.
    #[error(transparent)]
    Parse(#[from] crate::parser::ParseError),
    /// A PDF could not be produced.
    #[error(transparent)]
    Pdf(#[from] crate::pdf::PdfError),
    /// The gradebook has no students (Python refuses with "Must call parse() first").
    #[error("the gradebook has no students to generate reports for")]
    NoStudents,
    /// No student has the requested number.
    #[error("no student with number {0}")]
    StudentNotFound(i64),
    /// The output folder could not be created.
    #[error("cannot create or write the output folder {path}: {source}")]
    OutputNotWritable {
        /// The output folder.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The operation was cancelled.
    #[error("operation cancelled")]
    Cancelled,
}

impl EngineError {
    /// The stable code for this error (see the table on [`EngineError`]).
    pub fn code(&self) -> ErrorCode {
        match self {
            EngineError::Read(e) => match e {
                ReadError::NotFound { .. } => ErrorCode::FileNotFound,
                ReadError::Permission { .. } => ErrorCode::FileLocked,
                ReadError::UnsupportedFormat { .. } => ErrorCode::UnsupportedFormat,
                ReadError::Corrupted { message, .. } if message.contains("No columns to parse") => {
                    ErrorCode::EmptySheet
                }
                ReadError::Corrupted { .. } | ReadError::GridRead { .. } => {
                    ErrorCode::UnsupportedFormat
                }
                ReadError::Empty => ErrorCode::EmptySheet,
                ReadError::SheetNotFound { .. } => ErrorCode::NoGradebookFound,
            },
            EngineError::Parse(_) | EngineError::NoStudents => ErrorCode::NoGradebookFound,
            EngineError::Pdf(crate::pdf::PdfError::Io(_))
            | EngineError::OutputNotWritable { .. } => ErrorCode::OutputNotWritable,
            EngineError::Pdf(_) => ErrorCode::PdfFailed,
            EngineError::StudentNotFound(_) => ErrorCode::Internal,
            EngineError::Cancelled => ErrorCode::Cancelled,
        }
    }
}
