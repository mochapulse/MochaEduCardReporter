//! `dump`: write the checkpoint tree the Python oracle (`tools/oracle/dump.py`)
//! writes, so the two trees can be compared. `intermediate.csv`,
//! `audit_report` and `engine_sha256` are intentionally not produced.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use mocha_engine::model::{FileMetadata, StudentGrades};
use mocha_engine::parser::{self, ParseError};
use mocha_engine::{pdf, reader, EngineError, Gradebook};
use serde::Serialize;

use crate::CliError;

/// Default timestamp, the oracle's `TIMESTAMP_ISO`.
const TIMESTAMP_ISO: &str = "2026-01-15 10:00:00";

#[derive(Serialize)]
struct FileEntry {
    file: String,
    sheets: Vec<String>,
    sheets_including_empty: Vec<String>,
}

#[derive(Serialize)]
struct CaseEntry {
    id: String,
    dir: String,
    source_file: String,
    sheet: Option<String>,
    ok: bool,
    status: Option<String>,
    student_count: usize,
}

#[derive(Serialize)]
struct Manifest {
    timestamp_iso: String,
    files: Vec<FileEntry>,
    cases: Vec<CaseEntry>,
}

#[derive(Serialize)]
struct Parsed<'a> {
    metadata: Option<&'a FileMetadata>,
    students: &'a [StudentGrades],
    warnings: &'a [String],
    error: Option<String>,
}

#[derive(Serialize)]
struct MetaOut {
    first_block_start: i64,
    first_block_end: i64,
    component_start: i64,
    component_end: i64,
    final_row: i64,
}

#[derive(Serialize)]
struct RowsOut {
    student_number: Option<i64>,
    student_name: String,
    rows: Vec<[String; 4]>,
    meta: MetaOut,
}

/// The oracle's `safe_id`: control characters and `/\:*?"<>|` become `-`.
fn safe_id(text: &str) -> String {
    text.chars()
        .map(|c| {
            if (c as u32) < 0x20 || "/\\:*?\"<>|".contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect()
}

fn io_err(e: impl std::fmt::Display) -> CliError {
    CliError::Other(e.to_string())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_err)?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(io_err)?;
    text.push('\n');
    std::fs::write(path, text).map_err(io_err)
}

/// Python exception text the oracle records for a parse failure.
fn parse_error_text(e: &ParseError) -> String {
    let kind = match e {
        ParseError::NoSupportedLayout | ParseError::IntegerTooLarge { .. } => "ValueError",
        ParseError::InvalidGradeNumber { .. } => "OverflowError",
        ParseError::IndexOutOfRange { .. } => "IndexError",
    };
    format!("{kind}: {e}")
}

fn run_case(
    source: &Path,
    sheet: Option<&str>,
    case_id: &str,
    case_dir: &Path,
    timestamp: &str,
) -> Result<CaseEntry, CliError> {
    if case_dir.exists() {
        std::fs::remove_dir_all(case_dir).map_err(io_err)?;
    }
    std::fs::create_dir_all(case_dir).map_err(io_err)?;
    let source_file = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut entry = CaseEntry {
        id: case_id.to_string(),
        dir: case_id.to_string(),
        source_file,
        sheet: sheet.map(str::to_string),
        ok: false,
        status: None,
        student_count: 0,
    };

    let read = match reader::read_grid(source, sheet) {
        Ok(r) => r,
        Err(e) => {
            entry.status = Some(e.status().to_string());
            write_case(case_dir, &entry)?;
            return Ok(entry);
        }
    };
    entry.ok = true;
    entry.status = Some("success".to_string());
    write_json(&case_dir.join("grid.json"), &read.grid)?;

    let parsed = match parser::parse(&read.grid, &read.csv_file_name) {
        Ok(p) => p,
        Err(e) => {
            write_json(
                &case_dir.join("parsed.json"),
                &Parsed {
                    metadata: None,
                    students: &[],
                    warnings: &[],
                    error: Some(parse_error_text(&e)),
                },
            )?;
            entry.status = Some("parse_error".to_string());
            write_case(case_dir, &entry)?;
            return Ok(entry);
        }
    };
    write_json(
        &case_dir.join("parsed.json"),
        &Parsed {
            metadata: Some(&parsed.metadata),
            students: &parsed.students,
            warnings: &parsed.warnings,
            error: None,
        },
    )?;
    entry.student_count = parsed.students.len();

    let rows: Vec<RowsOut> = parsed
        .students
        .iter()
        .map(|s| {
            let (rows, m) = pdf::build_student_rows(&parsed.metadata, s);
            RowsOut {
                student_number: s.student_number,
                student_name: s.student_name.clone(),
                rows,
                meta: MetaOut {
                    first_block_start: m.first_block_start,
                    first_block_end: m.first_block_end,
                    component_start: m.component_start,
                    component_end: m.component_end,
                    final_row: m.final_row,
                },
            }
        })
        .collect();
    write_json(&case_dir.join("rows.json"), &rows)?;

    let pdf_dir = case_dir.join("pdf");
    let book = Gradebook {
        source: source.to_path_buf(),
        sheet: sheet.map(str::to_string),
        csv_file_name: read.csv_file_name,
        metadata: parsed.metadata,
        students: parsed.students,
        warnings: parsed.warnings,
    };
    let cancel = AtomicBool::new(false);
    match mocha_engine::generate_reports(
        &book,
        None,
        &pdf_dir,
        Some(timestamp),
        &mut |_| {},
        &cancel,
    ) {
        Ok(outcome) => {
            if let Some(f) = outcome.failures.first() {
                return Err(CliError::Other(format!(
                    "{case_id}: PDF failed for {}: {}",
                    f.student_name, f.message
                )));
            }
        }
        Err(EngineError::NoStudents) => std::fs::create_dir_all(&pdf_dir).map_err(io_err)?,
        Err(e) => return Err(e.into()),
    }
    write_case(case_dir, &entry)?;
    Ok(entry)
}

fn write_case(case_dir: &Path, entry: &CaseEntry) -> Result<(), CliError> {
    write_json(&case_dir.join("case.json"), entry)
}

/// Runs every `.xls`/`.xlsx`/`.csv` directly in `corpus` and writes the oracle layout to `out`.
pub fn run(corpus: &Path, out: &Path, date: Option<&str>) -> Result<(), CliError> {
    let timestamp = date.unwrap_or(TIMESTAMP_ISO);
    std::fs::create_dir_all(out).map_err(io_err)?;
    let mut files: Vec<_> = std::fs::read_dir(corpus)
        .map_err(io_err)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .map(|x| x.to_string_lossy().to_lowercase())
                    .is_some_and(|x| matches!(x.as_str(), "xls" | "xlsx" | "csv"))
        })
        .collect();
    files.sort();

    let mut manifest = Manifest {
        timestamp_iso: timestamp.to_string(),
        files: Vec::new(),
        cases: Vec::new(),
    };
    let mut seen = std::collections::HashSet::new();
    for path in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_csv = path
            .extension()
            .is_some_and(|x| x.to_string_lossy().eq_ignore_ascii_case("csv"));
        let targets: Vec<(Option<String>, String)> = if is_csv {
            vec![(None, stem)]
        } else {
            let sheets = reader::get_sheet_names(&path, false);
            manifest.files.push(FileEntry {
                file: name.clone(),
                sheets: sheets.clone(),
                sheets_including_empty: reader::get_sheet_names(&path, true),
            });
            sheets
                .into_iter()
                .map(|s| (Some(s.clone()), format!("{stem}__{s}")))
                .collect()
        };
        for (sheet, raw_id) in targets {
            let case_id = safe_id(&raw_id);
            if !seen.insert(case_id.clone()) {
                return Err(CliError::Other(format!("case id collision: {case_id}")));
            }
            let entry = run_case(
                &path,
                sheet.as_deref(),
                &case_id,
                &out.join(&case_id),
                timestamp,
            )?;
            eprintln!(
                "{}: ok={} status={} students={}",
                case_id,
                entry.ok,
                entry.status.as_deref().unwrap_or("none"),
                entry.student_count
            );
            manifest.cases.push(entry);
        }
    }
    write_json(&out.join("manifest.json"), &manifest)
}
