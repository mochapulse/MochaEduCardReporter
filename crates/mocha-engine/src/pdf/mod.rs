//! PDF report generation: a re-implementation of the ReportLab Platypus behaviors
//! used by `PDFGenerator` in `legacy/src/libs/excel_analyzer.py` (see
//! `PLAN_REDO_CODE.md` section 3.4), emitted with `pdf-writer`.

pub mod document;
pub mod fonts;
pub mod metrics;
pub mod paragraph;
pub mod report;
pub mod table;
pub mod writer;

pub use report::{build_student_rows, generate_student_pdf, render_student_pdf, RowMeta};

/// Errors of the PDF layer.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    /// Paragraph markup ReportLab would reject or that this port does not support.
    #[error("paragraph markup error: {0}")]
    Markup(String),
    /// A flowable could not be placed (`LayoutError` in ReportLab).
    #[error("layout error: {0}")]
    Layout(String),
    /// Filesystem error while writing the PDF.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}
