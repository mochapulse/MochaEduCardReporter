//! End-to-end tests of the orchestrator, from the source fixtures to PDFs,
//! checked against the recorded Python output in `fixtures/expected/`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use mocha_engine::model::{FileMetadata, StudentGrades};
use mocha_engine::{
    generate_reports, inspect, list_sheets, preview_student, EngineError, ErrorCode,
};
use serde::Deserialize;

const TIMESTAMP: &str = "2026-01-15 10:00:00";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

#[derive(Deserialize)]
struct Case {
    id: String,
    dir: String,
    source_file: String,
    sheet: Option<String>,
    ok: bool,
    student_count: usize,
}

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Parsed {
    metadata: FileMetadata,
    students: Vec<StudentGrades>,
    warnings: Vec<String>,
}

fn manifest() -> Manifest {
    let text =
        std::fs::read_to_string(fixtures().join("expected/manifest.json")).expect("manifest");
    serde_json::from_str(&text).expect("manifest json")
}

fn pdf_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read pdf dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn source(case: &Case) -> PathBuf {
    fixtures().join(&case.source_file)
}

fn ok_cases() -> Vec<Case> {
    manifest().cases.into_iter().filter(|c| c.ok).collect()
}

fn not_cancelled() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn inspect_matches_recorded_parsed_json() {
    for case in manifest().cases {
        let result = inspect(&source(&case), case.sheet.as_deref());
        if !case.ok {
            assert!(result.is_err(), "{} should fail", case.id);
            continue;
        }
        let book = result.unwrap_or_else(|e| panic!("{}: {e}", case.id));
        let text = std::fs::read_to_string(
            fixtures()
                .join("expected")
                .join(&case.dir)
                .join("parsed.json"),
        )
        .expect("parsed.json");
        let expected: Parsed = serde_json::from_str(&text).expect("parsed json");
        assert_eq!(book.metadata, expected.metadata, "{} metadata", case.id);
        assert_eq!(book.students, expected.students, "{} students", case.id);
        assert_eq!(book.warnings, expected.warnings, "{} warnings", case.id);
        // IndexMap equality ignores order: compare the serialized text too.
        assert_eq!(
            serde_json::to_string(&book.metadata).expect("ser"),
            serde_json::to_string(&expected.metadata).expect("ser"),
            "{} metadata key order",
            case.id
        );
        assert_eq!(
            serde_json::to_string(&book.students).expect("ser"),
            serde_json::to_string(&expected.students).expect("ser"),
            "{} students key order",
            case.id
        );
        assert_eq!(book.students.len(), case.student_count);
    }
}

#[test]
fn generate_matches_recorded_pdf_names_and_progress() {
    for case in ok_cases() {
        let book = inspect(&source(&case), case.sheet.as_deref()).expect("inspect");
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = tmp.path().join("nested/out");
        let mut calls = Vec::new();
        let outcome = generate_reports(
            &book,
            None,
            &out,
            Some(TIMESTAMP),
            &mut |p| calls.push(p.clone()),
            &not_cancelled(),
        )
        .unwrap_or_else(|e| panic!("{}: {e}", case.id));
        let expected = pdf_names(&fixtures().join("expected").join(&case.dir).join("pdf"));
        assert_eq!(pdf_names(&out), expected, "{} file names", case.id);
        assert_eq!(outcome.files.len(), expected.len());
        assert!(outcome.failures.is_empty() && !outcome.cancelled);
        assert_eq!(
            calls.len(),
            book.students.len(),
            "{} progress calls",
            case.id
        );
        for (i, p) in calls.iter().enumerate() {
            assert_eq!((p.done, p.total), (i + 1, book.students.len()));
            assert!(p.pdf_path.is_some() && p.error.is_none());
        }
        for file in &outcome.files {
            let bytes = std::fs::read(file).expect("read pdf");
            assert!(bytes.starts_with(b"%PDF"), "{} not a PDF", file.display());
        }
    }
}

#[test]
fn failing_cases_report_the_right_code() {
    for case in manifest().cases.into_iter().filter(|c| !c.ok) {
        let err = inspect(&source(&case), case.sheet.as_deref()).expect_err("must fail");
        assert_eq!(err.code(), ErrorCode::EmptySheet, "{}: {err}", case.id);
    }
}

fn first_ok_book() -> mocha_engine::Gradebook {
    let case = ok_cases().into_iter().next().expect("an ok case");
    inspect(&source(&case), case.sheet.as_deref()).expect("inspect")
}

#[test]
fn selection_subset() {
    let book = first_ok_book();
    let tmp = tempfile::tempdir().expect("tempdir");
    let wanted: Vec<i64> = book
        .students
        .iter()
        .skip(1)
        .take(2)
        .filter_map(|s| s.student_number)
        .collect();
    let mut calls = 0;
    let outcome = generate_reports(
        &book,
        Some(&wanted),
        tmp.path(),
        Some(TIMESTAMP),
        &mut |p| {
            calls += 1;
            assert_eq!(p.total, 2);
        },
        &not_cancelled(),
    )
    .expect("generate");
    assert_eq!(outcome.files.len(), 2);
    assert_eq!(calls, 2);
    assert_eq!(pdf_names(tmp.path()).len(), 2);
}

#[test]
fn cancellation_keeps_written_files() {
    let book = first_ok_book();
    let tmp = tempfile::tempdir().expect("tempdir");
    let cancel = AtomicBool::new(false);
    let outcome = generate_reports(
        &book,
        None,
        tmp.path(),
        Some(TIMESTAMP),
        &mut |p| {
            if p.done == 2 {
                cancel.store(true, Ordering::SeqCst);
            }
        },
        &cancel,
    )
    .expect("generate");
    assert!(outcome.cancelled);
    assert_eq!(outcome.files.len(), 2);
    assert_eq!(pdf_names(tmp.path()).len(), 2);
}

#[test]
fn preview_equals_generated_file() {
    let book = first_ok_book();
    let student = &book.students[0];
    let number = student.student_number.expect("number");
    let bytes = preview_student(&book, number, Some(TIMESTAMP)).expect("preview");
    let tmp = tempfile::tempdir().expect("tempdir");
    let outcome = generate_reports(
        &book,
        Some(&[number]),
        tmp.path(),
        Some(TIMESTAMP),
        &mut |_| {},
        &not_cancelled(),
    )
    .expect("generate");
    assert_eq!(outcome.files.len(), 1);
    assert_eq!(std::fs::read(&outcome.files[0]).expect("read"), bytes);
}

#[test]
fn unknown_student_number() {
    let book = first_ok_book();
    let err = preview_student(&book, 9999, None).expect_err("must fail");
    assert!(matches!(err, EngineError::StudentNotFound(9999)));
    assert_eq!(err.code(), ErrorCode::Internal);
}

#[test]
fn output_dir_not_writable() {
    let book = first_ok_book();
    let tmp = tempfile::tempdir().expect("tempdir");
    let file = tmp.path().join("regular_file");
    std::fs::write(&file, b"x").expect("write");
    let err = generate_reports(
        &book,
        None,
        &file.join("sub"),
        None,
        &mut |_| {},
        &not_cancelled(),
    )
    .expect_err("must fail");
    assert_eq!(err.code(), ErrorCode::OutputNotWritable);
}

#[test]
fn no_students_is_no_gradebook_found() {
    let mut book = first_ok_book();
    book.students.clear();
    let tmp = tempfile::tempdir().expect("tempdir");
    let err = generate_reports(&book, None, tmp.path(), None, &mut |_| {}, &not_cancelled())
        .expect_err("must fail");
    assert_eq!(err.code(), ErrorCode::NoGradebookFound);
}

#[test]
fn missing_source_and_unsupported_extension() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let err = inspect(&tmp.path().join("nope.xlsx"), None).expect_err("missing");
    assert_eq!(err.code(), ErrorCode::FileNotFound);
    let err = inspect(&tmp.path().join("nope.csv"), None).expect_err("missing csv");
    assert_eq!(err.code(), ErrorCode::FileNotFound);
    let odd = tmp.path().join("notes.txt");
    std::fs::write(&odd, b"a,b\n1,2\n").expect("write");
    let err = inspect(&odd, None).expect_err("unsupported");
    assert_eq!(err.code(), ErrorCode::UnsupportedFormat);
}

#[test]
fn error_codes_serialize_to_plan_strings() {
    let json = serde_json::to_string(&ErrorCode::OutputNotWritable).expect("ser");
    assert_eq!(json, "\"OUTPUT_NOT_WRITABLE\"");
    assert_eq!(ErrorCode::NoGradebookFound.as_str(), "NO_GRADEBOOK_FOUND");
}

#[test]
fn list_sheets_counts_match_manifest() {
    let manifest = manifest();
    let mut files: Vec<&str> = manifest
        .cases
        .iter()
        .filter(|c| c.sheet.is_some())
        .map(|c| c.source_file.as_str())
        .collect();
    files.dedup();
    assert!(!files.is_empty());
    for file in files {
        let sheets = list_sheets(&fixtures().join(file));
        let expected: Vec<&Case> = manifest
            .cases
            .iter()
            .filter(|c| c.source_file == file && c.sheet.is_some())
            .collect();
        assert_eq!(sheets.len(), expected.len(), "{file}");
        for (info, case) in sheets.iter().zip(expected) {
            assert_eq!(Some(&info.name), case.sheet.as_ref());
            assert_eq!(info.student_count, Some(case.student_count), "{}", case.id);
        }
    }
    // A CSV source has no sheets.
    let csv = manifest
        .cases
        .iter()
        .find(|c| c.sheet.is_none())
        .expect("csv case");
    assert!(list_sheets(&source(csv)).is_empty());
}
