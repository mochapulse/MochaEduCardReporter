//! The error type every command returns.
//!
//! It serializes as `{ code, message, detail }`. `code` is one of the engine's
//! stable [`ErrorCode`] strings (plan section 8); the frontend maps it to a
//! Spanish message with a suggested action. `message` is a short, stable
//! English summary of the code and `detail` carries the technical text (the
//! engine's error message, a path, ...) behind the "Ver detalles" button.

use mocha_engine::{EngineError, ErrorCode};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Mirror of [`mocha_engine::ErrorCode`] that can be described to TypeScript.
///
/// The serialized strings are identical to `ErrorCode::as_str`; a unit test
/// keeps both enums in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppErrorCode {
    /// The source file does not exist.
    FileNotFound,
    /// The source file cannot be opened (locked or permission denied).
    FileLocked,
    /// Not a supported spreadsheet.
    UnsupportedFormat,
    /// No recognizable gradebook, or nothing to generate.
    NoGradebookFound,
    /// The file or sheet is empty.
    EmptySheet,
    /// The output folder cannot be created or written.
    OutputNotWritable,
    /// A PDF could not be laid out.
    PdfFailed,
    /// The user cancelled the operation.
    Cancelled,
    /// Anything else: bugs, rejected paths, stale ids, bad arguments.
    Internal,
}

impl From<ErrorCode> for AppErrorCode {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::FileNotFound => Self::FileNotFound,
            ErrorCode::FileLocked => Self::FileLocked,
            ErrorCode::UnsupportedFormat => Self::UnsupportedFormat,
            ErrorCode::NoGradebookFound => Self::NoGradebookFound,
            ErrorCode::EmptySheet => Self::EmptySheet,
            ErrorCode::OutputNotWritable => Self::OutputNotWritable,
            ErrorCode::PdfFailed => Self::PdfFailed,
            ErrorCode::Cancelled => Self::Cancelled,
            ErrorCode::Internal => Self::Internal,
        }
    }
}

impl AppErrorCode {
    /// The code as the string the frontend receives.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileNotFound => ErrorCode::FileNotFound,
            Self::FileLocked => ErrorCode::FileLocked,
            Self::UnsupportedFormat => ErrorCode::UnsupportedFormat,
            Self::NoGradebookFound => ErrorCode::NoGradebookFound,
            Self::EmptySheet => ErrorCode::EmptySheet,
            Self::OutputNotWritable => ErrorCode::OutputNotWritable,
            Self::PdfFailed => ErrorCode::PdfFailed,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Internal => ErrorCode::Internal,
        }
        .as_str()
    }

    /// Short stable English summary of the code.
    fn summary(self) -> &'static str {
        match self {
            Self::FileNotFound => "The file does not exist.",
            Self::FileLocked => "The file cannot be opened; it may be open in another program.",
            Self::UnsupportedFormat => "The file is not a supported spreadsheet.",
            Self::NoGradebookFound => "No gradebook was found in the file.",
            Self::EmptySheet => "The file or sheet is empty.",
            Self::OutputNotWritable => "The output folder cannot be written.",
            Self::PdfFailed => "A report card could not be generated.",
            Self::Cancelled => "The operation was cancelled.",
            Self::Internal => "Unexpected error.",
        }
    }
}

/// Error returned by every command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, thiserror::Error)]
#[error("{}: {message}", .code.as_str())]
pub struct AppError {
    /// Stable machine-readable code.
    pub code: AppErrorCode,
    /// Short English summary of the code.
    pub message: String,
    /// Technical detail, when there is any.
    pub detail: Option<String>,
}

impl AppError {
    /// Builds an error with the code's standard message and a technical detail.
    pub fn new(code: AppErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            message: code.summary().to_string(),
            detail: Some(detail.into()),
        }
    }

    /// An `INTERNAL` error.
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(AppErrorCode::Internal, detail)
    }

    /// A `FILE_NOT_FOUND` error for `path`.
    pub fn file_not_found(path: &std::path::Path) -> Self {
        Self::new(
            AppErrorCode::FileNotFound,
            format!("not found: {}", path.display()),
        )
    }

    /// An `OUTPUT_NOT_WRITABLE` error.
    pub fn not_writable(detail: impl Into<String>) -> Self {
        Self::new(AppErrorCode::OutputNotWritable, detail)
    }

    /// Error for a path the frontend may not read or open.
    pub fn path_not_allowed(path: &std::path::Path) -> Self {
        Self::new(
            AppErrorCode::Internal,
            format!("path not allowed: {}", path.display()),
        )
    }
}

impl From<EngineError> for AppError {
    fn from(e: EngineError) -> Self {
        Self::new(e.code().into(), e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mocha_engine::error::ReadError;

    const ALL: [ErrorCode; 9] = [
        ErrorCode::FileNotFound,
        ErrorCode::FileLocked,
        ErrorCode::UnsupportedFormat,
        ErrorCode::NoGradebookFound,
        ErrorCode::EmptySheet,
        ErrorCode::OutputNotWritable,
        ErrorCode::PdfFailed,
        ErrorCode::Cancelled,
        ErrorCode::Internal,
    ];

    #[test]
    fn every_engine_code_round_trips_with_the_same_string() {
        for code in ALL {
            let app: AppErrorCode = code.into();
            assert_eq!(app.as_str(), code.as_str());
            let json = serde_json::to_string(&app).unwrap();
            assert_eq!(json, format!("\"{}\"", code.as_str()));
        }
    }

    #[test]
    fn engine_errors_map_to_their_codes() {
        let e: AppError = EngineError::NoStudents.into();
        assert_eq!(e.code, AppErrorCode::NoGradebookFound);
        let e: AppError = EngineError::Cancelled.into();
        assert_eq!(e.code, AppErrorCode::Cancelled);
        let e: AppError = EngineError::Read(ReadError::NotFound {
            path: "x.csv".into(),
            status: "failed",
        })
        .into();
        assert_eq!(e.code, AppErrorCode::FileNotFound);
        assert!(e.detail.as_deref().is_some_and(|d| d.contains("x.csv")));
        let e: AppError = EngineError::StudentNotFound(3).into();
        assert_eq!(e.code, AppErrorCode::Internal);
    }

    #[test]
    fn serializes_as_code_message_detail() {
        let e = AppError::new(AppErrorCode::PdfFailed, "boom");
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["code"], "PDF_FAILED");
        assert_eq!(v["message"], AppErrorCode::PdfFailed.summary());
        assert_eq!(v["detail"], "boom");
        let none = AppError {
            code: AppErrorCode::Internal,
            message: "m".into(),
            detail: None,
        };
        assert!(serde_json::to_value(none).unwrap()["detail"].is_null());
    }
}
