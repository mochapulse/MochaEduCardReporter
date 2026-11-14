//! Orchestrator: port of `GradebookProcessor` (py lines 1603-1873) split into
//! steps a UI can drive: [`inspect`] (convert + parse), then
//! [`generate_reports`] (per-student generation with progress and
//! cancellation) or [`preview_student`].
//!
//! File-name collisions: two students that produce the same `NN_Name.pdf`
//! overwrite each other, exactly like the Python engine (the later student
//! wins). Both paths are still listed in [`GenerateOutcome::files`], as Python's
//! `pdf_files` does.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::error::{EngineError, ErrorCode};
use crate::model::{FileMetadata, StudentGrades};
use crate::{parser, pdf, reader};

/// A gradebook that has been read and parsed (Python: `convert()` + `parse()`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gradebook {
    /// The source file.
    pub source: PathBuf,
    /// The sheet that was read (Excel only).
    pub sheet: Option<String>,
    /// Name of the intermediate CSV as Python has it.
    pub csv_file_name: String,
    /// Extracted metadata.
    pub metadata: FileMetadata,
    /// Students in sheet order.
    pub students: Vec<StudentGrades>,
    /// The parser's warnings.
    pub warnings: Vec<String>,
}

/// A usable sheet of a workbook with its student count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SheetInfo {
    /// Sheet name.
    pub name: String,
    /// Number of students, when the sheet could be inspected.
    pub student_count: Option<usize>,
    /// Why the sheet could not be inspected.
    pub error: Option<String>,
}

/// Usable sheets (`get_sheet_names`, `include_empty = false`), each with its
/// student count. A CSV source (or an unreadable workbook) gives an empty
/// list, like Python.
pub fn list_sheets(source: &Path) -> Vec<SheetInfo> {
    reader::get_sheet_names(source, false)
        .into_iter()
        .map(|name| match inspect(source, Some(&name)) {
            Ok(book) => SheetInfo {
                name,
                student_count: Some(book.students.len()),
                error: None,
            },
            Err(e) => SheetInfo {
                name,
                student_count: None,
                error: Some(e.to_string()),
            },
        })
        .collect()
}

/// Python: `GradebookProcessor(source, sheet).convert()` then `parse()`.
///
/// # Errors
///
/// [`EngineError::Read`] or [`EngineError::Parse`].
pub fn inspect(source: &Path, sheet: Option<&str>) -> Result<Gradebook, EngineError> {
    let read = reader::read_grid(source, sheet)?;
    let parsed = parser::parse(&read.grid, &read.csv_file_name)?;
    Ok(Gradebook {
        source: source.to_path_buf(),
        sheet: sheet.map(str::to_string),
        csv_file_name: read.csv_file_name,
        metadata: parsed.metadata,
        students: parsed.students,
        warnings: parsed.warnings,
    })
}

/// Progress after one student was handled (generated or failed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// Students handled so far, including this one.
    pub done: usize,
    /// Students selected.
    pub total: usize,
    /// Number of the student just handled.
    pub student_number: Option<i64>,
    /// Name of the student just handled.
    pub student_name: String,
    /// The PDF written, when generation succeeded.
    pub pdf_path: Option<PathBuf>,
    /// The failure message, when it failed.
    pub error: Option<String>,
}

/// A student whose report could not be generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StudentFailure {
    /// Number of the student.
    pub student_number: Option<i64>,
    /// Name of the student.
    pub student_name: String,
    /// Human-readable failure message.
    pub message: String,
    /// Stable error code of the failure.
    pub code: ErrorCode,
}

/// Result of [`generate_reports`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GenerateOutcome {
    /// PDFs written, in student order.
    pub files: Vec<PathBuf>,
    /// Students that failed (generation continued past them).
    pub failures: Vec<StudentFailure>,
    /// True when `cancel` was raised before all students were handled.
    pub cancelled: bool,
}

/// Generates one PDF per selected student, in sheet order.
///
/// `selection`: `None` = all students; `Some(numbers)` = only students whose
/// `student_number` is listed (unknown numbers are ignored). The output
/// directory is created with its parents. `on_progress` runs after each
/// student. `cancel` is checked before each student; on cancel the files
/// already written are kept and listed. A failure for one student does not
/// stop the others; it is recorded in [`GenerateOutcome::failures`].
///
/// # Errors
///
/// [`EngineError::NoStudents`] (code `NO_GRADEBOOK_FOUND`) when the gradebook
/// has no students, like Python's "Must call parse() first";
/// [`EngineError::OutputNotWritable`] when the output directory cannot be
/// created.
pub fn generate_reports(
    gradebook: &Gradebook,
    selection: Option<&[i64]>,
    output_dir: &Path,
    timestamp_iso: Option<&str>,
    on_progress: &mut dyn FnMut(&Progress),
    cancel: &AtomicBool,
) -> Result<GenerateOutcome, EngineError> {
    if gradebook.students.is_empty() {
        return Err(EngineError::NoStudents);
    }
    std::fs::create_dir_all(output_dir).map_err(|source| EngineError::OutputNotWritable {
        path: output_dir.display().to_string(),
        source,
    })?;

    let selected: Vec<&StudentGrades> = gradebook
        .students
        .iter()
        .filter(|s| match selection {
            None => true,
            Some(numbers) => s.student_number.is_some_and(|n| numbers.contains(&n)),
        })
        .collect();
    let total = selected.len();
    let mut outcome = GenerateOutcome {
        files: Vec::new(),
        failures: Vec::new(),
        cancelled: false,
    };

    for (index, student) in selected.into_iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            outcome.cancelled = true;
            break;
        }
        let result =
            pdf::generate_student_pdf(&gradebook.metadata, student, output_dir, timestamp_iso);
        let mut progress = Progress {
            done: index + 1,
            total,
            student_number: student.student_number,
            student_name: student.student_name.clone(),
            pdf_path: None,
            error: None,
        };
        match result {
            Ok(path) => {
                progress.pdf_path = Some(path.clone());
                outcome.files.push(path);
            }
            Err(e) => {
                let err = EngineError::from(e);
                progress.error = Some(err.to_string());
                outcome.failures.push(StudentFailure {
                    student_number: student.student_number,
                    student_name: student.student_name.clone(),
                    message: err.to_string(),
                    code: err.code(),
                });
            }
        }
        on_progress(&progress);
    }
    Ok(outcome)
}

/// One student's PDF as bytes (for the "Ver ejemplo" preview).
///
/// # Errors
///
/// [`EngineError::StudentNotFound`] when no student has that number, or a
/// [`EngineError::Pdf`] failure.
pub fn preview_student(
    gradebook: &Gradebook,
    student_number: i64,
    timestamp_iso: Option<&str>,
) -> Result<Vec<u8>, EngineError> {
    let student = gradebook
        .students
        .iter()
        .find(|s| s.student_number == Some(student_number))
        .ok_or(EngineError::StudentNotFound(student_number))?;
    Ok(pdf::render_student_pdf(
        &gradebook.metadata,
        student,
        timestamp_iso,
    )?)
}
