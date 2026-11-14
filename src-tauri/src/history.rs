//! History of past generations and the "recent files" list (plan 7.5, 7.6).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;
use crate::persist::{load_or_default, save, KeyValueStore};

const HISTORY_KEY: &str = "history";
const RECENT_KEY: &str = "recent_files";
/// Maximum number of history entries kept.
pub const HISTORY_CAP: usize = 50;
/// Maximum number of recent files kept.
pub const RECENT_CAP: usize = 5;

/// A past generation, as stored and as shown in "Boletines anteriores".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    /// Stable identifier (use it with `remove_history_entry`).
    pub id: String,
    /// When the generation happened, local time, `YYYY-MM-DDTHH:MM:SS`.
    pub created_at: String,
    /// The date printed on the report cards, `YYYY-MM-DD`.
    pub report_date: String,
    /// File name of the gradebook.
    pub source_file_name: String,
    /// Full path of the gradebook.
    pub source_path: String,
    /// Sheet that was read (Excel only).
    pub sheet: Option<String>,
    /// Subject as read from the file.
    pub subject: String,
    /// Group as read from the file.
    pub group: String,
    /// Period as read from the file.
    pub period: String,
    /// Folder the PDFs were written to.
    pub output_folder: String,
    /// The PDFs written.
    pub pdf_files: Vec<String>,
    /// Number of reports generated.
    pub generated_count: u32,
    /// Number of students that failed.
    pub failed_count: u32,
    /// Whether `output_folder` still exists. Recomputed on every read; a
    /// missing folder is flagged, never deleted from the history.
    #[serde(default)]
    pub folder_exists: bool,
}

/// A history entry to record (the store assigns id, time and existence flag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct NewHistoryEntry {
    /// The date printed on the report cards, `YYYY-MM-DD`.
    pub report_date: String,
    /// File name of the gradebook.
    pub source_file_name: String,
    /// Full path of the gradebook.
    pub source_path: String,
    /// Sheet that was read (Excel only).
    pub sheet: Option<String>,
    /// Subject as read from the file.
    pub subject: String,
    /// Group as read from the file.
    pub group: String,
    /// Period as read from the file.
    pub period: String,
    /// Folder the PDFs were written to.
    pub output_folder: String,
    /// The PDFs written.
    pub pdf_files: Vec<String>,
    /// Number of students that failed.
    pub failed_count: u32,
}

fn load_entries(store: &dyn KeyValueStore) -> Vec<HistoryEntry> {
    load_or_default(store, HISTORY_KEY)
}

fn write_err(e: String) -> AppError {
    AppError::internal(format!("cannot save to the app store: {e}"))
}

/// Records a generation, newest first, capped at [`HISTORY_CAP`].
///
/// An entry for the same output folder is updated in place (keeping its id)
/// and its PDF list is merged with the new one: a retry of a few students
/// into the same folder extends the entry rather than duplicating it. Merged
/// paths that no longer exist are dropped.
///
/// # Errors
///
/// `INTERNAL` when the store cannot be written.
pub fn add(
    store: &dyn KeyValueStore,
    new: NewHistoryEntry,
    now_iso: &str,
) -> Result<HistoryEntry, AppError> {
    let mut entries = load_entries(store);
    let existing_pos = entries
        .iter()
        .position(|e| e.output_folder == new.output_folder);
    let existing = existing_pos.map(|i| entries.remove(i));

    let mut pdf_files = new.pdf_files.clone();
    if let Some(old) = &existing {
        for p in &old.pdf_files {
            if !pdf_files.contains(p) && Path::new(p).exists() {
                pdf_files.push(p.clone());
            }
        }
    }
    pdf_files.sort();
    let id = match existing {
        Some(old) => old.id,
        None => fresh_id(now_iso, &entries),
    };
    let entry = HistoryEntry {
        id,
        created_at: now_iso.to_string(),
        report_date: new.report_date,
        source_file_name: new.source_file_name,
        source_path: new.source_path,
        sheet: new.sheet,
        subject: new.subject,
        group: new.group,
        period: new.period,
        generated_count: u32::try_from(pdf_files.len()).unwrap_or(u32::MAX),
        pdf_files,
        failed_count: new.failed_count,
        output_folder: new.output_folder,
        folder_exists: true,
    };
    entries.insert(0, entry.clone());
    entries.truncate(HISTORY_CAP);
    save(store, HISTORY_KEY, &entries).map_err(write_err)?;
    Ok(entry)
}

fn fresh_id(now_iso: &str, entries: &[HistoryEntry]) -> String {
    let base: String = now_iso.chars().filter(char::is_ascii_digit).collect();
    let mut n = 0u32;
    loop {
        let candidate = format!("{base}-{n}");
        if entries.iter().all(|e| e.id != candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// All entries, newest first, with `folder_exists` recomputed.
pub fn list(store: &dyn KeyValueStore) -> Vec<HistoryEntry> {
    let mut entries = load_entries(store);
    for e in &mut entries {
        e.folder_exists = Path::new(&e.output_folder).is_dir();
    }
    entries
}

/// Removes one entry (unknown ids are ignored).
///
/// # Errors
///
/// `INTERNAL` when the store cannot be written.
pub fn remove(store: &dyn KeyValueStore, id: &str) -> Result<(), AppError> {
    let mut entries = load_entries(store);
    let before = entries.len();
    entries.retain(|e| e.id != id);
    if entries.len() != before {
        save(store, HISTORY_KEY, &entries).map_err(write_err)?;
    }
    Ok(())
}

/// Output folders recorded in the history (roots `read_pdf` may serve).
pub fn folders(store: &dyn KeyValueStore) -> Vec<PathBuf> {
    load_entries(store)
        .into_iter()
        .map(|e| PathBuf::from(e.output_folder))
        .collect()
}

/// A recently opened gradebook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RecentFile {
    /// Full path.
    pub path: String,
    /// File name, for display.
    pub file_name: String,
    /// When it was last used, local time, `YYYY-MM-DDTHH:MM:SS`.
    pub last_used: String,
    /// Whether the file still exists. Recomputed on every read.
    #[serde(default)]
    pub exists: bool,
}

fn load_recent(store: &dyn KeyValueStore) -> Vec<RecentFile> {
    load_or_default(store, RECENT_KEY)
}

/// Marks `path` as just used: moves it to the front, keeps the last
/// [`RECENT_CAP`].
///
/// # Errors
///
/// `INTERNAL` when the store cannot be written.
pub fn add_recent(store: &dyn KeyValueStore, path: &str, now_iso: &str) -> Result<(), AppError> {
    let mut files = load_recent(store);
    files.retain(|f| f.path != path);
    let file_name = Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned());
    files.insert(
        0,
        RecentFile {
            path: path.to_string(),
            file_name,
            last_used: now_iso.to_string(),
            exists: true,
        },
    );
    files.truncate(RECENT_CAP);
    save(store, RECENT_KEY, &files).map_err(write_err)
}

/// The recent files, newest first, with `exists` recomputed.
pub fn list_recent(store: &dyn KeyValueStore) -> Vec<RecentFile> {
    let mut files = load_recent(store);
    for f in &mut files {
        f.exists = Path::new(&f.path).is_file();
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::MemoryStore;

    fn new_entry(folder: &str, pdfs: &[&str]) -> NewHistoryEntry {
        NewHistoryEntry {
            report_date: "2026-11-14".into(),
            source_file_name: "notas.csv".into(),
            source_path: "/x/notas.csv".into(),
            sheet: None,
            subject: "FRANÇAIS IV".into(),
            group: "02".into(),
            period: "2025-2".into(),
            output_folder: folder.into(),
            pdf_files: pdfs.iter().map(|s| (*s).to_string()).collect(),
            failed_count: 0,
        }
    }

    #[test]
    fn add_list_remove() {
        let mem = MemoryStore::new();
        let a = add(
            &mem,
            new_entry("/nope/a", &["/nope/a/1.pdf"]),
            "2026-11-14T10:00:00",
        )
        .unwrap();
        let b = add(&mem, new_entry("/nope/b", &[]), "2026-11-14T10:00:00").unwrap();
        assert_ne!(a.id, b.id);
        let all = list(&mem);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, b.id, "newest first");
        assert!(
            !all[0].folder_exists,
            "missing folders are flagged, not removed"
        );
        assert_eq!(all[1].generated_count, 1);
        remove(&mem, &a.id).unwrap();
        remove(&mem, "unknown").unwrap();
        assert_eq!(list(&mem).len(), 1);
    }

    #[test]
    fn history_is_capped() {
        let mem = MemoryStore::new();
        for i in 0..(HISTORY_CAP + 7) {
            add(
                &mem,
                new_entry(&format!("/nope/{i}"), &[]),
                "2026-11-14T10:00:00",
            )
            .unwrap();
        }
        let all = list(&mem);
        assert_eq!(all.len(), HISTORY_CAP);
        assert_eq!(all[0].output_folder, format!("/nope/{}", HISTORY_CAP + 6));
    }

    #[test]
    fn same_folder_is_merged_and_existing_flag_computed() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        let p1 = dir.path().join("01_A.pdf");
        let p2 = dir.path().join("02_B.pdf");
        std::fs::write(&p1, b"x").unwrap();
        std::fs::write(&p2, b"x").unwrap();
        let mem = MemoryStore::new();
        let first = add(
            &mem,
            new_entry(&folder, &[&p1.to_string_lossy()]),
            "2026-11-14T10:00:00",
        )
        .unwrap();
        let second = add(
            &mem,
            new_entry(&folder, &[&p2.to_string_lossy()]),
            "2026-11-14T11:00:00",
        )
        .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.generated_count, 2);
        let all = list(&mem);
        assert_eq!(all.len(), 1);
        assert!(all[0].folder_exists);
        assert_eq!(all[0].created_at, "2026-11-14T11:00:00");
        assert_eq!(folders(&mem), vec![PathBuf::from(&folder)]);
    }

    #[test]
    fn recent_files_dedupe_cap_and_exists() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.xlsx");
        std::fs::write(&real, b"x").unwrap();
        let mem = MemoryStore::new();
        add_recent(&mem, &real.to_string_lossy(), "2026-11-14T10:00:00").unwrap();
        for i in 0..6 {
            add_recent(&mem, &format!("/missing/f{i}.csv"), "2026-11-14T10:01:00").unwrap();
        }
        let list_now = list_recent(&mem);
        assert_eq!(list_now.len(), RECENT_CAP);
        assert_eq!(list_now[0].file_name, "f5.csv");
        assert!(!list_now[0].exists);
        // Re-adding moves to front and dedupes.
        add_recent(&mem, "/missing/f2.csv", "2026-11-14T12:00:00").unwrap();
        let l = list_recent(&mem);
        assert_eq!(l[0].path, "/missing/f2.csv");
        assert_eq!(l.iter().filter(|f| f.path == "/missing/f2.csv").count(), 1);
        // The real file fell off the cap of 5; re-add to check exists = true.
        add_recent(&mem, &real.to_string_lossy(), "2026-11-14T13:00:00").unwrap();
        assert!(list_recent(&mem)[0].exists);
    }

    #[test]
    fn corrupt_store_value_does_not_break_reads() {
        let mem = MemoryStore::new();
        mem.set(HISTORY_KEY, serde_json::json!("garbage")).unwrap();
        assert!(list(&mem).is_empty());
        add(&mem, new_entry("/nope/z", &[]), "2026-11-14T10:00:00").unwrap();
        assert_eq!(list(&mem).len(), 1);
    }
}
