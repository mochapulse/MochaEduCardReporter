//! Phase 1 regression tests for the PDF layout engine. Constants were verified
//! against ReportLab 4.4.10 output (see PLAN_REDO_CODE.md, checkpoint D); nothing
//! here needs Python.

use indexmap::IndexMap;
use mocha_engine::model::{FileMetadata, StudentGrades};
use mocha_engine::pdf::{build_student_rows, render_student_pdf, PdfError};

fn metadata(first: usize, comps: usize, label: impl Fn(usize) -> String) -> FileMetadata {
    let mut grade_labels = IndexMap::new();
    for i in 1..=first + comps {
        grade_labels.insert(i.to_string(), label(i));
    }
    FileMetadata {
        file_name: "mini.csv".into(),
        university_name: "UNIVERSITÉ DE TEST".into(),
        program_name: "LICENCE & MASTER".into(),
        materia_name: "FRANÇAIS I".into(),
        group_name: "01".into(),
        professor_name: "JOSÉ PÉREZ".into(),
        period_name: "2025".into(),
        first_twenty_grade_count: first,
        first_twenty_grade_numbers: (1..=first).map(|i| i.to_string()).collect(),
        component_grade_numbers: (first + 1..=first + comps).map(|i| i.to_string()).collect(),
        grade_labels,
        first_block_weight_label: "20%".into(),
        component_weight_labels: IndexMap::new(),
    }
}

fn student(meta: &FileMetadata) -> StudentGrades {
    let mut first = IndexMap::new();
    for n in &meta.first_twenty_grade_numbers {
        first.insert(format!("{n} - {}", meta.grade_labels[n]), "4.5".to_string());
    }
    let mut comp = IndexMap::new();
    let mut weighted = IndexMap::new();
    weighted.insert(
        format!("1-{} weighted 20%", meta.first_twenty_grade_count),
        "0.7".to_string(),
    );
    for n in &meta.component_grade_numbers {
        comp.insert(format!("{n} - {}", meta.grade_labels[n]), "3.5".to_string());
        weighted.insert(format!("{n} weighted 20%"), "0.7".to_string());
    }
    StudentGrades {
        student_number: Some(7),
        student_code: "123".into(),
        student_name: "Élève Test".into(),
        abse: "2.0".into(),
        first_twenty_grades: first,
        component_grades: comp,
        weighted_components: weighted,
        definitive_grade: "3.9".into(),
    }
}

/// All `1 0 0 1 X Y cm` translations in the (uncompressed) content streams.
fn translations(pdf: &[u8]) -> Vec<(f64, f64)> {
    let text = String::from_utf8_lossy(pdf);
    text.lines()
        .filter_map(|l| {
            let l = l.strip_suffix(" cm")?.strip_prefix("1 0 0 1 ")?;
            let mut it = l.split(' ');
            Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
        })
        .collect()
}

fn page_count(pdf: &[u8]) -> usize {
    String::from_utf8_lossy(pdf)
        .lines()
        .filter(|l| l.trim() == "/Type /Page")
        .count()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn mini_student_layout_matches_reportlab() {
    let long = "EXPRESSION ÉCRITE. Atelier de rédaction collective sur les courants littéraires du siècle et leur influence moderne. Le 2 mai.";
    let mut meta = metadata(2, 1, |i| match i {
        1 => long.to_string(),
        2 => "Quiz. Court. Le 3 mai.".to_string(),
        _ => "EXAMEN FINAL. Le 4 juin.".to_string(),
    });
    meta.first_twenty_grade_count = 2;
    let st = student(&meta);
    let pdf = render_student_pdf(&meta, &st, Some("2026-01-15 10:00:00")).expect("render");
    assert!(pdf.starts_with(b"%PDF"));
    assert_eq!(page_count(&pdf), 1);

    let t = translations(&pdf);
    // First flowable (date line) origin: left margin + 6 pt padding, top - 6 pt - 10 pt.
    assert!(
        close(t[0].0, 34.34646) && close(t[0].1, 803.2126),
        "{:?}",
        t[0]
    );
    // Table origin: x = frame x + (frame width - 470.5512) / 2; y = bottom of the table
    // (rows 14 + 29 + 29 + 20 + 11 = 103 pt high, top at 684.7087).
    assert!(
        t.iter()
            .any(|&(x, y)| close(x, 62.3622) && close(y, 581.7087)),
        "{t:?}"
    );
    let text = String::from_utf8_lossy(&pdf);
    // Header strings: centred in the 10 mm column, baseline = height - 14 + 6.
    assert!(text.contains("9.507728 95 Tm"));
    // Row 1 paragraph cell (3 lines, 29 pt row): "1" drawn at x = 2, y = 70 (VALIGN MIDDLE).
    assert!(t.iter().any(|&(x, y)| close(x, 2.0) && close(y, 70.0)));
    // Spanned "%" cell centred vertically over the two first-block rows.
    assert!(t.iter().any(|&(x, y)| close(x, 376.1732) && close(y, 55.5)));
}

#[test]
fn rows_match_python_for_the_mini_student() {
    let meta = metadata(2, 1, |i| match i {
        1 => "EXPRESSION ÉCRITE. Atelier de rédaction. Le 2 mai.".to_string(),
        2 => "Quiz. Court. Le 3 mai.".to_string(),
        _ => "EXAMEN FINAL. Le 4 juin.".to_string(),
    });
    let (rows, m) = build_student_rows(&meta, &student(&meta));
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows[0][0],
        "EXPRESSION ÉCRITE.<br/>Atelier de rédaction.<br/>Le 2 mai."
    );
    assert_eq!(rows[0][3], "0.7");
    assert_eq!(rows[1][3], "");
    assert_eq!(rows[3], ["NOTE DÉFINITIVE", "", "", "3.9"]);
    assert_eq!(
        (
            m.first_block_end,
            m.component_start,
            m.component_end,
            m.final_row
        ),
        (1, 2, 2, 3)
    );
}

#[test]
fn table_splits_with_repeated_header_across_pages() {
    // Oracle (ReportLab) produced 3 pages for this shape: 6 first-block rows + 30 components.
    let label = |i: usize| {
        format!("EXPRESSION ÉCRITE. Atelier de rédaction collective sur les courants littéraires du XIXe siècle et leur influence sur la poésie moderne. Le {} octobre.", i % 28 + 1)
    };
    let meta = metadata(6, 30, label);
    let pdf =
        render_student_pdf(&meta, &student(&meta), Some("2026-01-15 10:00:00")).expect("render");
    assert_eq!(page_count(&pdf), 3);
}

#[test]
fn unsplittable_first_block_is_a_layout_error_like_reportlab() {
    // ReportLab raises LayoutError (first block spans cannot be split) for this input.
    let label = |i: usize| {
        format!("EXPRESSION ÉCRITE. Atelier de rédaction collective sur les courants littéraires du XIXe siècle et leur influence sur la poésie moderne. Le {} octobre.", i % 28 + 1)
    };
    let meta = metadata(45, 4, label);
    let err = render_student_pdf(&meta, &student(&meta), None).expect_err("must fail");
    assert!(matches!(err, PdfError::Layout(_)));
}

#[test]
fn checkpoint_c_rows_match_oracle_rows_json() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected");
    assert!(root.is_dir(), "fixtures/expected missing");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("read_dir") {
        let dir = entry.expect("entry").path();
        let (parsed, rows) = (dir.join("parsed.json"), dir.join("rows.json"));
        if !parsed.is_file() || !rows.is_file() {
            continue;
        }
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(parsed).expect("read")).expect("json");
        if parsed["metadata"].is_null() {
            continue;
        }
        let meta: FileMetadata = serde_json::from_value(parsed["metadata"].clone()).expect("meta");
        let students: Vec<StudentGrades> =
            serde_json::from_value(parsed["students"].clone()).expect("students");
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(rows).expect("read")).expect("json");
        for (st, exp) in students.iter().zip(expected.as_array().expect("array")) {
            let (rows, m) = build_student_rows(&meta, st);
            let got: Vec<Vec<String>> = rows.iter().map(|r| r.to_vec()).collect();
            let want: Vec<Vec<String>> = serde_json::from_value(exp["rows"].clone()).expect("rows");
            assert_eq!(got, want, "rows of {}", st.student_name);
            let em = &exp["meta"];
            assert_eq!(
                m.first_block_start,
                em["first_block_start"].as_i64().expect("i")
            );
            assert_eq!(
                m.first_block_end,
                em["first_block_end"].as_i64().expect("i")
            );
            assert_eq!(
                m.component_start,
                em["component_start"].as_i64().expect("i")
            );
            assert_eq!(m.component_end, em["component_end"].as_i64().expect("i"));
            assert_eq!(m.final_row, em["final_row"].as_i64().expect("i"));
            checked += 1;
        }
    }
    assert!(checked > 50, "only {checked} students checked");
}
