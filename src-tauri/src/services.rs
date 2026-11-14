//! Command-level logic, independent of the Tauri runtime so that it can be
//! unit-tested: every `#[tauri::command]` in [`crate::commands`] is a thin
//! wrapper around one method of [`Services`].

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use mocha_engine::Progress;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppErrorCode};
use crate::history::{self, HistoryEntry, NewHistoryEntry, RecentFile};
use crate::paths::{self, OutputDirStatus};
use crate::persist::KeyValueStore;
use crate::settings::{self, Settings};
use crate::state::AppState;

/// Largest PDF `read_pdf` will serve.
const MAX_PDF_BYTES: u64 = 200 * 1024 * 1024;

/// Folders the app works with.
#[derive(Debug, Clone)]
pub struct Dirs {
    /// Where "Ver ejemplo" PDFs are written (always readable by the frontend).
    pub preview_dir: PathBuf,
    /// The log folder (openable, not readable).
    pub log_dir: PathBuf,
    /// The user's Documents folder, when known.
    pub documents_dir: Option<PathBuf>,
    /// The user's home folder, when known.
    pub home_dir: Option<PathBuf>,
}

/// A usable sheet of a workbook with its student count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct SheetInfoView {
    /// Sheet name.
    pub name: String,
    /// Number of students, when the sheet could be inspected.
    pub student_count: Option<u32>,
    /// Why the sheet could not be inspected.
    pub error: Option<String>,
}

/// One grade column: its key (as in the file) and the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct GradeEntry {
    /// Column key, in file order.
    pub key: String,
    /// Grade text as the engine holds it.
    pub value: String,
}

/// One student of the gradebook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct StudentView {
    /// Number in the list (the key used by `preview_student` / `generate_reports`).
    pub number: Option<i32>,
    /// Full name.
    pub name: String,
    /// Student code.
    pub code: String,
    /// Absences.
    pub absences: String,
    /// Final grade.
    pub definitive_grade: String,
    /// Grades of the first block.
    pub first_twenty_grades: Vec<GradeEntry>,
    /// Grades of the component block.
    pub component_grades: Vec<GradeEntry>,
    /// Weighted component grades.
    pub weighted_components: Vec<GradeEntry>,
}

/// Header data of the gradebook ("Materia, Grupo, Profesor, Periodo").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct MetadataView {
    /// University.
    pub university: String,
    /// Program.
    pub program: String,
    /// Subject ("Materia").
    pub subject: String,
    /// Group ("Grupo").
    pub group: String,
    /// Teacher ("Profesor").
    pub professor: String,
    /// Period ("Periodo").
    pub period: String,
}

/// Everything step 1 ("Revisar") shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct GradebookView {
    /// Id to pass to `preview_student` and `generate_reports`.
    pub id: String,
    /// Path of the gradebook.
    pub source_path: String,
    /// File name of the gradebook.
    pub source_file_name: String,
    /// The sheet read (Excel only).
    pub sheet: Option<String>,
    /// Header data.
    pub metadata: MetadataView,
    /// Students, in sheet order.
    pub students: Vec<StudentView>,
    /// Parser warnings, as the engine words them.
    pub warnings: Vec<String>,
    /// `<Materia> – Grupo <n> – <Periodo>`, safe for Windows and Linux.
    pub suggested_folder_name: String,
}

/// Progress after one student, streamed over the channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ProgressEvent {
    /// Students handled so far, including this one.
    pub done: u32,
    /// Students selected.
    pub total: u32,
    /// Number of the student just handled.
    pub student_number: Option<i32>,
    /// Name of the student just handled.
    pub student_name: String,
    /// The PDF written, when generation succeeded.
    pub pdf_path: Option<String>,
    /// The failure message, when it failed.
    pub error: Option<String>,
}

/// A report card that was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct GeneratedFile {
    /// Number of the student.
    pub student_number: Option<i32>,
    /// Name of the student.
    pub student_name: String,
    /// Full path of the PDF.
    pub path: String,
}

/// A student whose report card could not be generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct FailureView {
    /// Number of the student.
    pub student_number: Option<i32>,
    /// Name of the student.
    pub student_name: String,
    /// Technical message.
    pub message: String,
    /// Stable code.
    pub code: AppErrorCode,
}

/// Result of `generate_reports`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct GenerateResult {
    /// The (canonical) folder the PDFs were written to.
    pub output_dir: String,
    /// PDFs written, in student order.
    pub files: Vec<GeneratedFile>,
    /// Students that failed.
    pub failures: Vec<FailureView>,
    /// True when the user cancelled before all students were handled.
    pub cancelled: bool,
    /// Id of the history entry recorded for this run (none when nothing was
    /// written or the history could not be saved).
    pub history_id: Option<String>,
}

/// Application and engine versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct AppInfo {
    /// App version.
    pub version: String,
    /// Engine crate version.
    pub engine_version: String,
    /// Folder holding the log files.
    pub log_dir: String,
}

fn to_i32(n: Option<i64>) -> Option<i32> {
    n.and_then(|v| i32::try_from(v).ok())
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn entries(map: &indexmap::IndexMap<String, String>) -> Vec<GradeEntry> {
    map.iter()
        .map(|(k, v)| GradeEntry {
            key: k.clone(),
            value: v.clone(),
        })
        .collect()
}

impl From<&Progress> for ProgressEvent {
    fn from(p: &Progress) -> Self {
        Self {
            done: to_u32(p.done),
            total: to_u32(p.total),
            student_number: to_i32(p.student_number),
            student_name: p.student_name.clone(),
            pdf_path: p
                .pdf_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            error: p.error.clone(),
        }
    }
}

/// Validates the `YYYY-MM-DD` date chosen in the UI and returns the form the
/// engine's date line expects (`YYYY-MM-DD HH:MM:SS`, as the legacy app
/// passed). The engine prints it as `DD/MM/YYYY`.
///
/// # Errors
///
/// `INTERNAL` when `date_iso` is not a valid `YYYY-MM-DD` date.
pub fn engine_timestamp(date_iso: &str) -> Result<String, AppError> {
    let s = date_iso.trim();
    let valid = s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok();
    if valid {
        Ok(format!("{s} 00:00:00"))
    } else {
        Err(AppError::internal(format!(
            "invalid date '{date_iso}', expected YYYY-MM-DD"
        )))
    }
}

fn existing_file(path: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(path);
    if p.is_file() {
        Ok(p)
    } else {
        Err(AppError::file_not_found(&p))
    }
}

/// Current local time as `YYYY-MM-DDTHH:MM:SS`.
fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Everything the commands need: state, persistence and folders.
pub struct Services {
    /// In-memory state.
    pub state: AppState,
    store: Arc<dyn KeyValueStore>,
    dirs: Dirs,
    /// Serializes read-modify-write cycles on the store.
    store_lock: Mutex<()>,
}

impl Services {
    /// Creates the services and prepares the preview folder (created, and
    /// emptied of PDFs left by a previous session).
    pub fn new(store: Arc<dyn KeyValueStore>, dirs: Dirs) -> Self {
        if let Err(e) = fs::create_dir_all(&dirs.preview_dir) {
            log::warn!(
                "cannot create the preview folder {}: {e}",
                dirs.preview_dir.display()
            );
        }
        if let Ok(read) = fs::read_dir(&dirs.preview_dir) {
            for entry in read.filter_map(Result::ok) {
                let p = entry.path();
                if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")) {
                    let _ = fs::remove_file(p);
                }
            }
        }
        Self {
            state: AppState::new(),
            store,
            dirs,
            store_lock: Mutex::new(()),
        }
    }

    fn persist_guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.store_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Folders `read_pdf` may serve: the preview folder, folders written this
    /// session and folders recorded in the history.
    pub fn read_roots(&self) -> Vec<PathBuf> {
        let mut roots = vec![self.dirs.preview_dir.clone()];
        roots.extend(self.state.session_roots());
        roots.extend(history::folders(self.store.as_ref()));
        roots
    }

    /// Folders `reveal_in_folder` / `open_externally` may act on: the read
    /// roots plus the log folder.
    pub fn open_roots(&self) -> Vec<PathBuf> {
        let mut roots = self.read_roots();
        roots.push(self.dirs.log_dir.clone());
        roots
    }

    /// `list_sheets`.
    ///
    /// # Errors
    ///
    /// `FILE_NOT_FOUND` when `path` is not a file.
    pub fn list_sheets(&self, path: &str) -> Result<Vec<SheetInfoView>, AppError> {
        let p = existing_file(path)?;
        Ok(mocha_engine::list_sheets(&p)
            .into_iter()
            .map(|s| SheetInfoView {
                name: s.name,
                student_count: s.student_count.map(to_u32),
                error: s.error,
            })
            .collect())
    }

    /// `inspect_gradebook`: reads and parses the file, keeps the result in
    /// the cache and returns what step 1 shows.
    ///
    /// # Errors
    ///
    /// `FILE_NOT_FOUND`, or the engine's code for the failure.
    pub fn inspect_gradebook(
        &self,
        path: &str,
        sheet: Option<String>,
    ) -> Result<GradebookView, AppError> {
        let p = existing_file(path)?;
        let book = mocha_engine::inspect(&p, sheet.as_deref())?;
        let meta = &book.metadata;
        let view_meta = MetadataView {
            university: meta.university_name.clone(),
            program: meta.program_name.clone(),
            subject: meta.materia_name.clone(),
            group: meta.group_name.clone(),
            professor: meta.professor_name.clone(),
            period: meta.period_name.clone(),
        };
        let students = book
            .students
            .iter()
            .map(|s| StudentView {
                number: to_i32(s.student_number),
                name: s.student_name.clone(),
                code: s.student_code.clone(),
                absences: s.abse.clone(),
                definitive_grade: s.definitive_grade.clone(),
                first_twenty_grades: entries(&s.first_twenty_grades),
                component_grades: entries(&s.component_grades),
                weighted_components: entries(&s.weighted_components),
            })
            .collect();
        let source_file_name = p
            .file_name()
            .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned());
        let suggested_folder_name =
            paths::suggested_folder_name(&view_meta.subject, &view_meta.group, &view_meta.period);
        let warnings = book.warnings.clone();
        let sheet = book.sheet.clone();
        let id = self.state.insert_gradebook(book);
        Ok(GradebookView {
            id,
            source_path: path.to_string(),
            source_file_name,
            sheet,
            metadata: view_meta,
            students,
            warnings,
            suggested_folder_name,
        })
    }

    /// `preview_student`: writes one PDF into the preview folder and returns
    /// its path.
    ///
    /// # Errors
    ///
    /// `INTERNAL` for an unknown id or a bad date, the engine's code for a
    /// layout failure, `OUTPUT_NOT_WRITABLE` when the file cannot be written.
    pub fn preview_student(
        &self,
        gradebook_id: &str,
        student_number: i32,
        date_iso: &str,
    ) -> Result<String, AppError> {
        let book = self.state.gradebook(gradebook_id)?;
        let ts = engine_timestamp(date_iso)?;
        let bytes = mocha_engine::preview_student(&book, i64::from(student_number), Some(&ts))?;
        fs::create_dir_all(&self.dirs.preview_dir).map_err(|e| {
            AppError::not_writable(format!("{}: {e}", self.dirs.preview_dir.display()))
        })?;
        let file = self
            .dirs
            .preview_dir
            .join(format!("preview_{gradebook_id}_{student_number:02}.pdf"));
        fs::write(&file, bytes)
            .map_err(|e| AppError::not_writable(format!("{}: {e}", file.display())))?;
        Ok(file.to_string_lossy().into_owned())
    }

    /// `generate_reports`: one PDF per selected student into `output_dir`,
    /// streaming one [`ProgressEvent`] per student, then recording the run
    /// in the history.
    ///
    /// # Errors
    ///
    /// `INTERNAL` for an unknown id, a bad date, a relative output path or a
    /// run already in progress; `OUTPUT_NOT_WRITABLE` when the folder cannot
    /// be created; `NO_GRADEBOOK_FOUND` when there is nothing to generate.
    pub fn generate_reports(
        &self,
        gradebook_id: &str,
        students: Option<Vec<i32>>,
        output_dir: &str,
        date_iso: &str,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<GenerateResult, AppError> {
        let book = self.state.gradebook(gradebook_id)?;
        let ts = engine_timestamp(date_iso)?;
        let out = PathBuf::from(output_dir);
        if output_dir.trim().is_empty() || !out.is_absolute() {
            return Err(AppError::internal(format!(
                "the output folder must be an absolute path: '{output_dir}'"
            )));
        }
        let guard = self.state.begin_generation()?;
        fs::create_dir_all(&out)
            .map_err(|e| AppError::not_writable(format!("{}: {e}", out.display())))?;
        let canonical = fs::canonicalize(&out).unwrap_or_else(|_| out.clone());
        self.state.allow_root(&canonical);

        let selection: Option<Vec<i64>> = students.map(|v| v.into_iter().map(i64::from).collect());
        let mut files: Vec<GeneratedFile> = Vec::new();
        let mut forward = |p: &Progress| {
            let ev = ProgressEvent::from(p);
            if let Some(path) = &ev.pdf_path {
                files.push(GeneratedFile {
                    student_number: ev.student_number,
                    student_name: ev.student_name.clone(),
                    path: path.clone(),
                });
            }
            on_progress(ev);
        };
        let outcome = mocha_engine::generate_reports(
            &book,
            selection.as_deref(),
            &out,
            Some(&ts),
            &mut forward,
            guard.cancel_flag(),
        )?;
        drop(guard);

        let failures: Vec<FailureView> = outcome
            .failures
            .iter()
            .map(|f| FailureView {
                student_number: to_i32(f.student_number),
                student_name: f.student_name.clone(),
                message: f.message.clone(),
                code: f.code.into(),
            })
            .collect();
        let history_id = if files.is_empty() {
            None
        } else {
            let new = NewHistoryEntry {
                report_date: date_iso.trim().to_string(),
                source_file_name: book
                    .source
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                source_path: book.source.to_string_lossy().into_owned(),
                sheet: book.sheet.clone(),
                subject: book.metadata.materia_name.clone(),
                group: book.metadata.group_name.clone(),
                period: book.metadata.period_name.clone(),
                output_folder: canonical.to_string_lossy().into_owned(),
                pdf_files: files.iter().map(|f| f.path.clone()).collect(),
                failed_count: to_u32(failures.len()),
            };
            match self.add_history_entry(new) {
                Ok(entry) => Some(entry.id),
                Err(e) => {
                    log::warn!("could not record the generation in the history: {e}");
                    None
                }
            }
        };
        Ok(GenerateResult {
            output_dir: canonical.to_string_lossy().into_owned(),
            files,
            failures,
            cancelled: outcome.cancelled,
            history_id,
        })
    }

    /// `cancel_generation`.
    pub fn cancel_generation(&self) {
        self.state.request_cancel();
    }

    /// `check_output_dir`.
    pub fn check_output_dir(&self, path: &str) -> OutputDirStatus {
        paths::check_output_dir(Path::new(path))
    }

    /// `default_output_base`.
    pub fn default_output_base(&self) -> String {
        let s = settings::load(self.store.as_ref());
        settings::default_output_base(
            &s,
            self.dirs.home_dir.as_deref(),
            self.dirs.documents_dir.as_deref(),
        )
        .to_string_lossy()
        .into_owned()
    }

    /// `read_pdf`: the bytes of a PDF inside an allowed folder.
    ///
    /// # Errors
    ///
    /// `INTERNAL` for a path outside the allowed folders, a non-PDF or an
    /// oversized file; `FILE_NOT_FOUND` when missing.
    pub fn read_pdf(&self, path: &str) -> Result<Vec<u8>, AppError> {
        let file = paths::resolve_allowed_pdf(Path::new(path), &self.read_roots())?;
        let len = fs::metadata(&file)
            .map_err(|e| AppError::internal(e.to_string()))?
            .len();
        if len > MAX_PDF_BYTES {
            return Err(AppError::internal(format!("PDF too large ({len} bytes)")));
        }
        fs::read(&file).map_err(|e| AppError::internal(format!("{}: {e}", file.display())))
    }

    /// Validates a path for `reveal_in_folder` / `open_externally` and
    /// returns its canonical form.
    ///
    /// # Errors
    ///
    /// As [`paths::resolve_allowed`].
    pub fn authorize_open(&self, path: &str) -> Result<PathBuf, AppError> {
        paths::resolve_allowed(Path::new(path), &self.open_roots())
    }

    /// The log folder.
    pub fn log_dir(&self) -> &Path {
        &self.dirs.log_dir
    }

    /// `get_settings`.
    pub fn get_settings(&self) -> Settings {
        settings::load(self.store.as_ref())
    }

    /// `save_settings`.
    ///
    /// # Errors
    ///
    /// `INTERNAL` when the store cannot be written.
    pub fn save_settings(&self, new: &Settings) -> Result<(), AppError> {
        let _g = self.persist_guard();
        settings::store(self.store.as_ref(), new)
    }

    /// `get_recent_files`.
    pub fn get_recent_files(&self) -> Vec<RecentFile> {
        history::list_recent(self.store.as_ref())
    }

    /// `add_recent_file`.
    ///
    /// # Errors
    ///
    /// `INTERNAL` when the store cannot be written.
    pub fn add_recent_file(&self, path: &str) -> Result<(), AppError> {
        let _g = self.persist_guard();
        history::add_recent(self.store.as_ref(), path, &now_iso())
    }

    /// `get_history`.
    pub fn get_history(&self) -> Vec<HistoryEntry> {
        history::list(self.store.as_ref())
    }

    /// `add_history_entry`.
    ///
    /// # Errors
    ///
    /// `INTERNAL` when the store cannot be written.
    pub fn add_history_entry(&self, entry: NewHistoryEntry) -> Result<HistoryEntry, AppError> {
        let _g = self.persist_guard();
        history::add(self.store.as_ref(), entry, &now_iso())
    }

    /// `remove_history_entry`.
    ///
    /// # Errors
    ///
    /// `INTERNAL` when the store cannot be written.
    pub fn remove_history_entry(&self, id: &str) -> Result<(), AppError> {
        let _g = self.persist_guard();
        history::remove(self.store.as_ref(), id)
    }

    /// `app_info`.
    pub fn app_info(&self) -> AppInfo {
        AppInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            // The engine inherits the workspace version; the crate exposes no
            // version constant of its own.
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            log_dir: self.dirs.log_dir.to_string_lossy().into_owned(),
        }
    }
}
