//! Command-level tests that drive the real engine, plus the bindings
//! freshness check.

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::AppErrorCode;
use crate::persist::MemoryStore;
use crate::services::{engine_timestamp, Dirs, ProgressEvent, Services};

const FIXTURE: &str = "NOTAS FRANCÉS 2025-2_FRANCAIS IV LE MATIN.csv";

fn fixture() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join(FIXTURE)
        .to_string_lossy()
        .into_owned()
}

fn services(root: &std::path::Path) -> Services {
    let dirs = Dirs {
        preview_dir: root.join("cache").join("preview"),
        log_dir: root.join("logs"),
        documents_dir: Some(root.join("Documents")),
        home_dir: Some(root.to_path_buf()),
    };
    Services::new(Arc::new(MemoryStore::new()), dirs)
}

#[test]
fn inspect_generate_history_and_read_with_the_real_engine() {
    let tmp = tempfile::tempdir().unwrap();
    let svc = services(tmp.path());

    let view = svc.inspect_gradebook(&fixture(), None).unwrap();
    assert_eq!(view.metadata.subject, "FRANÇAIS IV");
    assert_eq!(
        view.suggested_folder_name,
        "FRANÇAIS IV \u{2013} Grupo 02 \u{2013} 2025"
    );
    assert!(!view.students.is_empty());
    assert!(view
        .students
        .iter()
        .all(|s| s.number.is_some() && !s.name.is_empty()));
    assert!(
        !view.warnings.is_empty(),
        "this fixture has a numbering warning"
    );
    let total = view.students.len();

    let out = tmp
        .path()
        .join("Boletines")
        .join(&view.suggested_folder_name);
    let mut events: Vec<ProgressEvent> = Vec::new();
    let result = svc
        .generate_reports(
            &view.id,
            None,
            &out.to_string_lossy(),
            "2026-11-14",
            &mut |e| events.push(e),
        )
        .unwrap();

    assert!(!result.cancelled);
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    assert_eq!(result.files.len(), total);
    assert_eq!(events.len(), total);
    assert_eq!(
        events.last().map(|e| (e.done, e.total)),
        Some((total as u32, total as u32))
    );
    for f in &result.files {
        assert!(std::path::Path::new(&f.path).is_file());
    }

    // History entry recorded by the run.
    let history = svc.get_history();
    assert_eq!(history.len(), 1);
    assert_eq!(Some(history[0].id.clone()), result.history_id);
    assert_eq!(history[0].generated_count as usize, total);
    assert_eq!(history[0].subject, "FRANÇAIS IV");
    assert_eq!(history[0].report_date, "2026-11-14");
    assert!(history[0].folder_exists);

    // The folder was registered: its PDFs can be read, others cannot.
    let bytes = svc.read_pdf(&result.files[0].path).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    let outside = tmp.path().join("other.pdf");
    std::fs::write(&outside, b"%PDF-1.4").unwrap();
    assert_eq!(
        svc.read_pdf(&outside.to_string_lossy()).unwrap_err().code,
        AppErrorCode::Internal
    );
    let sneaky = out.join("..").join("..").join("other.pdf");
    assert!(svc.read_pdf(&sneaky.to_string_lossy()).is_err());
    assert!(svc.authorize_open(&out.to_string_lossy()).is_ok());
    assert!(svc.authorize_open(&tmp.path().to_string_lossy()).is_err());

    // Output status now reports the existing PDFs.
    let status = svc.check_output_dir(&out.to_string_lossy());
    assert!(status.exists && status.writable);
    assert_eq!(status.pdf_count as usize, total);
    assert!(status.alternative_name.ends_with(" (2)"));
}

#[test]
fn subset_cancel_and_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let svc = services(tmp.path());
    let view = svc.inspect_gradebook(&fixture(), None).unwrap();
    let numbers: Vec<i32> = view.students.iter().filter_map(|s| s.number).collect();
    let out = tmp.path().join("out");

    let subset = vec![numbers[0], numbers[1]];
    let r = svc
        .generate_reports(
            &view.id,
            Some(subset),
            &out.to_string_lossy(),
            "2026-11-14",
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(r.files.len(), 2);

    // Cancel after the first student.
    let out2 = tmp.path().join("out2");
    let r = svc
        .generate_reports(
            &view.id,
            None,
            &out2.to_string_lossy(),
            "2026-11-14",
            &mut |e| {
                if e.done == 1 {
                    svc.cancel_generation();
                }
            },
        )
        .unwrap();
    assert!(r.cancelled);
    assert_eq!(r.files.len(), 1, "files written before the cancel are kept");
    assert!(r.history_id.is_some());
    // A new run is allowed afterwards.
    assert!(svc
        .generate_reports(
            &view.id,
            Some(vec![numbers[0]]),
            &out2.to_string_lossy(),
            "2026-11-14",
            &mut |_| {}
        )
        .is_ok());

    // Preview lands in the preview folder and is readable.
    let p = svc
        .preview_student(&view.id, numbers[0], "2026-11-14")
        .unwrap();
    assert!(p.contains("preview"));
    let pdf = svc.read_pdf(&p).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let printed = String::from_utf8_lossy(&pdf);
    assert!(
        printed.contains("Date: 14/11/2026"),
        "the date line must print as DD/MM/YYYY"
    );
    assert_eq!(
        svc.preview_student(&view.id, 9999, "2026-11-14")
            .unwrap_err()
            .code,
        AppErrorCode::Internal
    );
}

#[test]
fn argument_errors_have_codes() {
    let tmp = tempfile::tempdir().unwrap();
    let svc = services(tmp.path());
    assert_eq!(
        svc.inspect_gradebook("/no/such/file.csv", None)
            .unwrap_err()
            .code,
        AppErrorCode::FileNotFound
    );
    assert_eq!(
        svc.list_sheets("/no/such/file.xlsx").unwrap_err().code,
        AppErrorCode::FileNotFound
    );
    let txt = tmp.path().join("notes.txt");
    std::fs::write(&txt, "hola").unwrap();
    assert_eq!(
        svc.inspect_gradebook(&txt.to_string_lossy(), None)
            .unwrap_err()
            .code,
        AppErrorCode::UnsupportedFormat
    );
    assert!(svc.preview_student("gb-404", 1, "2026-11-14").is_err());
    let view = svc.inspect_gradebook(&fixture(), None).unwrap();
    assert!(svc
        .generate_reports(&view.id, None, "relative/dir", "2026-11-14", &mut |_| {})
        .is_err());
    assert!(svc
        .generate_reports(
            &view.id,
            None,
            &tmp.path().join("x").to_string_lossy(),
            "14/11/2026",
            &mut |_| {}
        )
        .is_err());
}

#[test]
fn date_is_validated_and_passed_in_legacy_form() {
    assert_eq!(
        engine_timestamp("2026-11-14").unwrap(),
        "2026-11-14 00:00:00"
    );
    assert!(engine_timestamp("2026-13-14").is_err());
    assert!(engine_timestamp("2026-1-4").is_err());
    assert!(engine_timestamp("").is_err());
}

#[test]
fn default_base_uses_settings_then_documents() {
    let tmp = tempfile::tempdir().unwrap();
    let svc = services(tmp.path());
    assert_eq!(
        svc.default_output_base(),
        tmp.path()
            .join("Documents")
            .join("Boletines")
            .to_string_lossy()
    );
    let mut s = svc.get_settings();
    s.base_folder = Some("/elegida".into());
    svc.save_settings(&s).unwrap();
    assert_eq!(svc.default_output_base(), "/elegida");
}

/// Fails when `src/lib/bindings.ts` is stale. Regenerate with
/// `UPDATE_BINDINGS=1 cargo test -p mocha-edu-card-reporter bindings`.
#[test]
fn bindings_are_up_to_date() {
    let tmp = tempfile::tempdir().unwrap();
    let fresh_path = tmp.path().join("bindings.ts");
    crate::specta_builder()
        .export(specta_typescript::Typescript::default(), &fresh_path)
        .unwrap();
    let fresh = std::fs::read_to_string(&fresh_path).unwrap();
    if std::env::var_os("UPDATE_BINDINGS").is_some() {
        std::fs::write(crate::BINDINGS_PATH, &fresh).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(crate::BINDINGS_PATH).unwrap_or_default();
    assert!(
        committed == fresh,
        "src/lib/bindings.ts is stale; run: UPDATE_BINDINGS=1 cargo test -p mocha-edu-card-reporter bindings"
    );
}
