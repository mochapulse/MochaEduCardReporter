//! mocha-engine: gradebook reading, parsing and PDF report generation.
//!
//! Behavior-parity port of `legacy/src/libs/excel_analyzer.py`.
//! See `PLAN_REDO_CODE.md` (section 2) for the parity rules.

pub mod error;
pub mod model;
pub mod parser;
pub mod patterns;
pub mod pdf;
pub mod processor;
pub mod pycompat;
pub mod reader;
pub mod text;

pub use error::{EngineError, ErrorCode};
pub use processor::{
    generate_reports, inspect, list_sheets, preview_student, GenerateOutcome, Gradebook, Progress,
    SheetInfo, StudentFailure,
};
