//! The `#[tauri::command]` surface. Each command is a thin wrapper: it moves
//! the work to a blocking thread (so the window never freezes) and calls the
//! matching method of [`Services`].

use std::sync::Arc;

use tauri::ipc::{Channel, InvokeResponseBody, IpcResponse};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::history::{HistoryEntry, NewHistoryEntry, RecentFile};
use crate::paths::OutputDirStatus;
use crate::services::{
    AppInfo, GenerateResult, GradebookView, ProgressEvent, Services, SheetInfoView,
};
use crate::settings::Settings;

/// Raw PDF bytes sent over IPC without JSON encoding. The frontend receives an
/// `ArrayBuffer`, which is what the generated bindings declare.
#[derive(Debug)]
pub struct PdfBytes(pub Vec<u8>);

impl IpcResponse for PdfBytes {
    fn body(self) -> tauri::Result<InvokeResponseBody> {
        Ok(InvokeResponseBody::Raw(self.0))
    }
}

impl specta::Type for PdfBytes {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta_typescript::define("ArrayBuffer").into()
    }
}

/// The managed state type every command receives.
pub type Shared<'a> = State<'a, Arc<Services>>;

/// Runs `f` on a blocking thread with the services, logging failures.
async fn blocking<T, F>(name: &'static str, services: &Shared<'_>, f: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&Services) -> Result<T, AppError> + Send + 'static,
{
    let services = Arc::clone(services.inner());
    let result = tauri::async_runtime::spawn_blocking(move || f(&services))
        .await
        .map_err(|e| AppError::internal(format!("{name}: worker thread failed: {e}")))?;
    if let Err(e) = &result {
        log::error!(
            "{name} failed: {e} ({})",
            e.detail.as_deref().unwrap_or("no detail")
        );
    }
    result
}

/// Usable sheets of a workbook with their student counts.
#[tauri::command]
#[specta::specta]
pub async fn list_sheets(
    services: Shared<'_>,
    path: String,
) -> Result<Vec<SheetInfoView>, AppError> {
    blocking("list_sheets", &services, move |s| s.list_sheets(&path)).await
}

/// Reads and parses a gradebook; the parsed data stays in memory under the
/// returned `id`.
#[tauri::command]
#[specta::specta]
pub async fn inspect_gradebook(
    services: Shared<'_>,
    path: String,
    sheet: Option<String>,
) -> Result<GradebookView, AppError> {
    blocking("inspect_gradebook", &services, move |s| {
        s.inspect_gradebook(&path, sheet)
    })
    .await
}

/// Writes one student's PDF to the preview folder and returns its path.
#[tauri::command]
#[specta::specta]
pub async fn preview_student(
    services: Shared<'_>,
    gradebook_id: String,
    student_number: i32,
    date_iso: String,
) -> Result<String, AppError> {
    blocking("preview_student", &services, move |s| {
        s.preview_student(&gradebook_id, student_number, &date_iso)
    })
    .await
}

/// Generates the report cards, streaming one event per student.
#[tauri::command]
#[specta::specta]
pub async fn generate_reports(
    services: Shared<'_>,
    gradebook_id: String,
    students: Option<Vec<i32>>,
    output_dir: String,
    date_iso: String,
    on_progress: Channel<ProgressEvent>,
) -> Result<GenerateResult, AppError> {
    blocking("generate_reports", &services, move |s| {
        s.generate_reports(
            &gradebook_id,
            students,
            &output_dir,
            &date_iso,
            &mut |event| {
                if let Err(e) = on_progress.send(event) {
                    log::warn!("progress channel closed: {e}");
                }
            },
        )
    })
    .await
}

/// Asks the running generation to stop after the current student.
#[tauri::command]
#[specta::specta]
pub fn cancel_generation(services: Shared<'_>) {
    services.cancel_generation();
}

/// Inspects a candidate output folder.
#[tauri::command]
#[specta::specta]
pub async fn check_output_dir(
    services: Shared<'_>,
    path: String,
) -> Result<OutputDirStatus, AppError> {
    blocking("check_output_dir", &services, move |s| {
        Ok(s.check_output_dir(&path))
    })
    .await
}

/// The folder report-card folders are created in by default.
#[tauri::command]
#[specta::specta]
pub fn default_output_base(services: Shared<'_>) -> String {
    services.default_output_base()
}

/// Raw bytes of a PDF inside the preview folder, a folder written this
/// session or a folder recorded in the history.
#[tauri::command]
#[specta::specta]
pub async fn read_pdf(services: Shared<'_>, path: String) -> Result<PdfBytes, AppError> {
    let bytes = blocking("read_pdf", &services, move |s| s.read_pdf(&path)).await?;
    Ok(PdfBytes(bytes))
}

/// The user settings.
#[tauri::command]
#[specta::specta]
pub fn get_settings(services: Shared<'_>) -> Settings {
    services.get_settings()
}

/// Saves the user settings.
#[tauri::command]
#[specta::specta]
pub fn save_settings(services: Shared<'_>, settings: Settings) -> Result<(), AppError> {
    services.save_settings(&settings)
}

/// The last 5 gradebooks opened.
#[tauri::command]
#[specta::specta]
pub fn get_recent_files(services: Shared<'_>) -> Vec<RecentFile> {
    services.get_recent_files()
}

/// Records a gradebook as just used.
#[tauri::command]
#[specta::specta]
pub fn add_recent_file(services: Shared<'_>, path: String) -> Result<(), AppError> {
    services.add_recent_file(&path)
}

/// Past generations, newest first.
#[tauri::command]
#[specta::specta]
pub fn get_history(services: Shared<'_>) -> Vec<HistoryEntry> {
    services.get_history()
}

/// Records a past generation (`generate_reports` already does this itself).
#[tauri::command]
#[specta::specta]
pub fn add_history_entry(
    services: Shared<'_>,
    entry: NewHistoryEntry,
) -> Result<HistoryEntry, AppError> {
    services.add_history_entry(entry)
}

/// Forgets one past generation (its files are not touched).
#[tauri::command]
#[specta::specta]
pub fn remove_history_entry(services: Shared<'_>, id: String) -> Result<(), AppError> {
    services.remove_history_entry(&id)
}

fn opener_error(e: &tauri_plugin_opener::Error) -> AppError {
    AppError::internal(format!("cannot open: {e}"))
}

/// Shows a file in the system file manager (a folder is simply opened).
#[tauri::command]
#[specta::specta]
pub fn reveal_in_folder(
    app: AppHandle,
    services: Shared<'_>,
    path: String,
) -> Result<(), AppError> {
    let target = services.authorize_open(&path)?;
    let opener = app.opener();
    let result = if target.is_dir() {
        opener.open_path(target.to_string_lossy(), None::<&str>)
    } else {
        opener.reveal_item_in_dir(&target)
    };
    result.map_err(|e| opener_error(&e))
}

/// Opens a file or folder with the system's default program.
#[tauri::command]
#[specta::specta]
pub fn open_externally(app: AppHandle, services: Shared<'_>, path: String) -> Result<(), AppError> {
    let target = services.authorize_open(&path)?;
    app.opener()
        .open_path(target.to_string_lossy(), None::<&str>)
        .map_err(|e| opener_error(&e))
}

/// Opens the log folder.
#[tauri::command]
#[specta::specta]
pub fn open_log_dir(app: AppHandle, services: Shared<'_>) -> Result<(), AppError> {
    let dir = services.log_dir();
    std::fs::create_dir_all(dir)
        .map_err(|e| AppError::internal(format!("{}: {e}", dir.display())))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| opener_error(&e))
}

/// App and engine versions.
#[tauri::command]
#[specta::specta]
pub fn app_info(services: Shared<'_>) -> AppInfo {
    services.app_info()
}
