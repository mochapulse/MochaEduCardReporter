//! Checkpoint B: parser output (metadata, students, warnings) against the
//! Python oracle, for the project fixtures and the synthetic parser cases.
//! Map comparisons are order-sensitive (key sequences are compared).

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use mocha_engine::model::{FileMetadata, Grid, StudentGrades};
use mocha_engine::parser::{parse, ParseError};
use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    dir: String,
    source_file: String,
    sheet: Option<String>,
    ok: bool,
}

#[derive(Deserialize)]
struct Parsed {
    metadata: Option<FileMetadata>,
    students: Vec<StudentGrades>,
    warnings: Vec<String>,
    error: Option<String>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `Path(source).stem` for the file names used by the oracle.
fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    }
}

/// The `csv_file` the Python engine passed to the parser for this case.
fn csv_file_for(case: &Case) -> String {
    match &case.sheet {
        Some(sheet) => format!("{}_{}.csv", stem(&case.source_file), sheet),
        None => case.source_file.clone(),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn diff_str(path: &str, exp: &str, got: &str) -> Option<String> {
    (exp != got).then(|| format!("{path}: expected {exp:?}, got {got:?}"))
}

/// Order-sensitive comparison of two maps; reports the first difference.
fn diff_map(
    path: &str,
    exp: &IndexMap<String, String>,
    got: &IndexMap<String, String>,
) -> Option<String> {
    let e: Vec<_> = exp.iter().collect();
    let g: Vec<_> = got.iter().collect();
    for i in 0..e.len().max(g.len()) {
        match (e.get(i), g.get(i)) {
            (Some(a), Some(b)) if a == b => {}
            (a, b) => return Some(format!("{path}[entry {i}]: expected {a:?}, got {b:?}")),
        }
    }
    None
}

fn diff_vec(path: &str, exp: &[String], got: &[String]) -> Option<String> {
    for i in 0..exp.len().max(got.len()) {
        if exp.get(i) != got.get(i) {
            return Some(format!(
                "{path}[{i}]: expected {:?}, got {:?}",
                exp.get(i),
                got.get(i)
            ));
        }
    }
    None
}

fn diff_metadata(e: &FileMetadata, g: &FileMetadata) -> Option<String> {
    let scalars = [
        ("file_name", &e.file_name, &g.file_name),
        ("university_name", &e.university_name, &g.university_name),
        ("program_name", &e.program_name, &g.program_name),
        ("materia_name", &e.materia_name, &g.materia_name),
        ("group_name", &e.group_name, &g.group_name),
        ("professor_name", &e.professor_name, &g.professor_name),
        ("period_name", &e.period_name, &g.period_name),
        (
            "first_block_weight_label",
            &e.first_block_weight_label,
            &g.first_block_weight_label,
        ),
    ];
    for (name, a, b) in scalars {
        if let Some(d) = diff_str(&format!("metadata.{name}"), a, b) {
            return Some(d);
        }
    }
    if e.first_twenty_grade_count != g.first_twenty_grade_count {
        return Some(format!(
            "metadata.first_twenty_grade_count: expected {}, got {}",
            e.first_twenty_grade_count, g.first_twenty_grade_count
        ));
    }
    diff_vec(
        "metadata.first_twenty_grade_numbers",
        &e.first_twenty_grade_numbers,
        &g.first_twenty_grade_numbers,
    )
    .or_else(|| {
        diff_vec(
            "metadata.component_grade_numbers",
            &e.component_grade_numbers,
            &g.component_grade_numbers,
        )
    })
    .or_else(|| diff_map("metadata.grade_labels", &e.grade_labels, &g.grade_labels))
    .or_else(|| {
        diff_map(
            "metadata.component_weight_labels",
            &e.component_weight_labels,
            &g.component_weight_labels,
        )
    })
}

fn diff_student(i: usize, e: &StudentGrades, g: &StudentGrades) -> Option<String> {
    let p = format!("students[{i}]");
    if e.student_number != g.student_number {
        return Some(format!(
            "{p}.student_number: expected {:?}, got {:?}",
            e.student_number, g.student_number
        ));
    }
    diff_str(
        &format!("{p}.student_code"),
        &e.student_code,
        &g.student_code,
    )
    .or_else(|| {
        diff_str(
            &format!("{p}.student_name"),
            &e.student_name,
            &g.student_name,
        )
    })
    .or_else(|| diff_str(&format!("{p}.abse"), &e.abse, &g.abse))
    .or_else(|| {
        diff_map(
            &format!("{p}.first_twenty_grades"),
            &e.first_twenty_grades,
            &g.first_twenty_grades,
        )
    })
    .or_else(|| {
        diff_map(
            &format!("{p}.component_grades"),
            &e.component_grades,
            &g.component_grades,
        )
    })
    .or_else(|| {
        diff_map(
            &format!("{p}.weighted_components"),
            &e.weighted_components,
            &g.weighted_components,
        )
    })
    .or_else(|| {
        diff_str(
            &format!("{p}.definitive_grade"),
            &e.definitive_grade,
            &g.definitive_grade,
        )
    })
}

/// Compare one case; `Some(message)` on the first difference.
fn check_case(root: &Path, case: &Case) -> Option<String> {
    let dir = root.join(&case.dir);
    let grid: Grid = read_json(&dir.join("grid.json"));
    let expected: Parsed = read_json(&dir.join("parsed.json"));
    let result = parse(&grid, &csv_file_for(case));

    if let Some(err) = &expected.error {
        return match result {
            Ok(_) => Some(format!("error: expected Python error {err:?}, got Ok")),
            Err(got) => {
                let matches = match &got {
                    ParseError::NoSupportedLayout => {
                        err.starts_with("ValueError: Could not find a supported")
                    }
                    ParseError::IntegerTooLarge { .. } => {
                        err.starts_with("ValueError: Exceeds the limit")
                    }
                    ParseError::IndexOutOfRange { .. } => {
                        err.starts_with("IndexError") || err.starts_with("KeyError")
                    }
                    ParseError::InvalidGradeNumber { .. } => err.starts_with("OverflowError"),
                };
                (!matches).then(|| format!("error: expected Python error {err:?}, got {got:?}"))
            }
        };
    }

    let got = match result {
        Ok(o) => o,
        Err(e) => return Some(format!("error: expected Ok, got {e:?}")),
    };
    let Some(exp_meta) = &expected.metadata else {
        return Some("expected has neither metadata nor error".to_string());
    };
    if let Some(d) = diff_metadata(exp_meta, &got.metadata) {
        return Some(d);
    }
    if expected.students.len() != got.students.len() {
        return Some(format!(
            "students.len: expected {}, got {}",
            expected.students.len(),
            got.students.len()
        ));
    }
    for (i, (e, g)) in expected.students.iter().zip(&got.students).enumerate() {
        if let Some(d) = diff_student(i, e, g) {
            return Some(d);
        }
    }
    diff_vec("warnings", &expected.warnings, &got.warnings)
}

fn run_manifest(root: &Path) -> usize {
    let manifest: Manifest = read_json(&root.join("manifest.json"));
    let mut failures = Vec::new();
    let mut checked = 0;
    for case in manifest.cases.iter().filter(|c| c.ok) {
        checked += 1;
        if let Some(msg) = check_case(root, case) {
            failures.push(format!("case {:?}: {msg}", case.id));
        }
    }
    assert!(
        failures.is_empty(),
        "checkpoint B mismatches:\n{}",
        failures.join("\n")
    );
    checked
}

#[test]
fn checkpoint_b_fixtures() {
    let n = run_manifest(&repo_root().join("fixtures/expected"));
    assert_eq!(n, 12, "expected the 12 parsable fixture cases");
}

#[test]
fn checkpoint_b_synthetic() {
    let n = run_manifest(&repo_root().join("fixtures/synthetic-parser/expected"));
    assert!(n >= 20, "synthetic corpus unexpectedly small: {n}");
}

#[test]
fn broken_numbering_warning_matches_python_test() {
    let root = repo_root().join("fixtures/synthetic-parser/expected");
    let grid: Grid = read_json(&root.join("broken_enumeration/grid.json"));
    let out = parse(&grid, "broken_enumeration.csv").expect("parses");
    assert_eq!(out.warnings.len(), 1);
    assert!(out.warnings[0].starts_with("Student enumeration corrected from row order: "));
    let numbers: Vec<_> = out
        .students
        .iter()
        .take(3)
        .map(|s| s.student_number)
        .collect();
    assert_eq!(numbers, vec![Some(1), Some(2), Some(3)]);
}

fn g(rows: &[&[&str]]) -> Grid {
    rows.iter()
        .map(|r| r.iter().map(|c| (*c).to_string()).collect())
        .collect()
}

/// Degenerate grids; the expected values were taken from Python
/// (`GradebookParser.parse` on the equivalent CSV) where Python can build the
/// grid at all.
#[test]
fn degenerate_grids_do_not_panic_and_match_python() {
    // Python: ValueError("Could not find a supported ...") for all of these.
    for grid in [
        g(&[&["a"]]),
        g(&[&[""]]),
        g(&[]),
        g(&[&[]]),
        g(&[&[], &[]]),
        g(&[&["a", "b"], &["c", "d"]]),
    ] {
        assert_eq!(
            parse(&grid, "x.csv"),
            Err(ParseError::NoSupportedLayout),
            "{grid:?}"
        );
    }

    // Header only: Python returns metadata and zero students.
    let out = parse(&g(&[&["No.", "CODE", "NOM", "ABSE"]]), "headeronly.csv").expect("ok");
    assert_eq!(out.metadata.file_name, "headeronly.csv");
    assert_eq!(out.metadata.university_name, "No.");
    assert_eq!(out.metadata.first_block_weight_label, "20%");
    assert!(out.students.is_empty() && out.warnings.is_empty());

    let out = parse(
        &g(&[
            &["x", "x", "x", "x", "moy"],
            &["No.", "CODE", "NOM", "ABSE", "1"],
        ]),
        "headeronly_moy.csv",
    )
    .expect("ok");
    assert_eq!(
        (
            out.metadata.university_name.as_str(),
            out.metadata.program_name.as_str()
        ),
        ("x", "No.")
    );
    assert!(out.students.is_empty());

    // Carnet header shorter than 5 columns: Python IndexError.
    assert!(matches!(
        parse(&g(&[&["No", "Item", "Note", "Moy"]]), "c.csv"),
        Err(ParseError::IndexOutOfRange { .. })
    ));

    // Carnet with the weighted column found at index 5.
    let out = parse(
        &g(&[
            &["No", "Item", "Note", "Moy", "x", "5"],
            &["1", "a", "2", "3", "4", "5"],
        ]),
        "carnet_wide.csv",
    )
    .expect("ok");
    assert_eq!(out.metadata.first_block_weight_label, "5%");
    assert_eq!(out.metadata.period_name, "carnet_wide");
    assert_eq!(out.students.len(), 1);
    assert_eq!(out.students[0].definitive_grade, "");
    assert_eq!(
        out.students[0]
            .first_twenty_grades
            .get("1 - a")
            .map(String::as_str),
        Some("2.0")
    );

    // One student row.
    let out = parse(
        &g(&[
            &["No.", "CODE", "NOM", "ABSE", "1", "Moy"],
            &["1", "c", "n", "0", "5", "5"],
        ]),
        "onestudent.csv",
    )
    .expect("ok");
    assert_eq!(out.students.len(), 1);
    assert_eq!(out.students[0].abse, "0.0");
    assert_eq!(
        out.students[0]
            .weighted_components
            .get("1-1 weighted 20%")
            .map(String::as_str),
        Some("")
    );
}

/// Deterministic pseudo-random tiny grids built from header-like tokens: the
/// parser must return (Ok or Err), never panic.
#[test]
fn random_tiny_grids_never_panic() {
    let tokens = [
        "",
        "",
        "No.",
        "N\u{b0}",
        "code",
        "nom",
        "\u{c9}tudiant",
        "abse",
        "ABSENCES",
        "moy",
        "1",
        "2.0",
        "d\u{e9}fin.",
        "item",
        "note",
        "note du cours",
        "1 abc",
        "0.2",
        "20%",
        "99999999999999999999",
        "\u{663}",
        "x",
        "Absences: 3",
    ];
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(state >> 33).unwrap_or(0)
    };
    for _ in 0..4000 {
        let rows = next() % 6;
        let cols = next() % 8;
        let grid: Grid = (0..rows)
            .map(|_| {
                (0..cols)
                    .map(|_| tokens[next() % tokens.len()].to_string())
                    .collect()
            })
            .collect();
        let _ = parse(&grid, "random_FRANCAIS.csv");
    }
    // Ragged grids as well.
    for _ in 0..1000 {
        let rows = next() % 6;
        let grid: Grid = (0..rows)
            .map(|_| {
                (0..next() % 8)
                    .map(|_| tokens[next() % tokens.len()].to_string())
                    .collect()
            })
            .collect();
        let _ = parse(&grid, "ragged.csv");
    }
}
