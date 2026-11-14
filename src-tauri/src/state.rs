//! In-memory application state: the parsed-gradebook cache, the cancel flag
//! of the running generation and the registry of folders this session may
//! serve PDFs from.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use mocha_engine::Gradebook;

use crate::error::AppError;

/// Gradebooks kept in memory; the oldest is dropped beyond this.
const CACHE_CAP: usize = 8;

/// Shared state, owned by [`crate::services::Services`].
#[derive(Debug, Default)]
pub struct AppState {
    gradebooks: Mutex<VecDeque<(String, Arc<Gradebook>)>>,
    next_id: AtomicU64,
    cancel: AtomicBool,
    running: AtomicBool,
    session_roots: Mutex<Vec<PathBuf>>,
}

/// Locks a mutex, recovering the data if a previous holder panicked (the
/// state stays usable; every value in it is valid on its own).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Marks a generation as running; dropping it marks it finished.
#[derive(Debug)]
pub struct RunGuard<'a> {
    state: &'a AppState,
}

impl RunGuard<'_> {
    /// The flag [`AppState::request_cancel`] raises.
    pub fn cancel_flag(&self) -> &AtomicBool {
        &self.state.cancel
    }
}

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        self.state.running.store(false, Ordering::SeqCst);
    }
}

impl AppState {
    /// An empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a parsed gradebook and returns its id.
    pub fn insert_gradebook(&self, book: Gradebook) -> String {
        let n = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let id = format!("gb-{n}");
        let mut cache = lock(&self.gradebooks);
        cache.push_back((id.clone(), Arc::new(book)));
        while cache.len() > CACHE_CAP {
            cache.pop_front();
        }
        id
    }

    /// Looks a gradebook up by id.
    ///
    /// # Errors
    ///
    /// `INTERNAL` when the id is unknown (never issued, or evicted).
    pub fn gradebook(&self, id: &str) -> Result<Arc<Gradebook>, AppError> {
        lock(&self.gradebooks)
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| Arc::clone(v))
            .ok_or_else(|| {
                AppError::internal(format!("unknown gradebook id '{id}'; open the file again"))
            })
    }

    /// Starts a generation run.
    ///
    /// # Errors
    ///
    /// `INTERNAL` when one is already running.
    pub fn begin_generation(&self) -> Result<RunGuard<'_>, AppError> {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(AppError::internal("a generation is already running"));
        }
        self.cancel.store(false, Ordering::SeqCst);
        Ok(RunGuard { state: self })
    }

    /// Raises the cancel flag of the running generation. Returns whether one
    /// was running.
    pub fn request_cancel(&self) -> bool {
        let running = self.running.load(Ordering::SeqCst);
        if running {
            self.cancel.store(true, Ordering::SeqCst);
        }
        running
    }

    /// Registers a folder whose PDFs the frontend may read this session.
    pub fn allow_root(&self, dir: &Path) {
        let mut roots = lock(&self.session_roots);
        if !roots.iter().any(|r| r == dir) {
            roots.push(dir.to_path_buf());
        }
    }

    /// The folders registered with [`AppState::allow_root`].
    pub fn session_roots(&self) -> Vec<PathBuf> {
        lock(&self.session_roots).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppErrorCode;
    use mocha_engine::model::FileMetadata;

    fn book(name: &str) -> Gradebook {
        Gradebook {
            source: PathBuf::from(name),
            sheet: None,
            csv_file_name: String::new(),
            metadata: FileMetadata {
                file_name: name.into(),
                university_name: String::new(),
                program_name: String::new(),
                materia_name: String::new(),
                group_name: String::new(),
                professor_name: String::new(),
                period_name: String::new(),
                first_twenty_grade_count: 0,
                first_twenty_grade_numbers: vec![],
                component_grade_numbers: vec![],
                grade_labels: Default::default(),
                first_block_weight_label: "20%".into(),
                component_weight_labels: Default::default(),
            },
            students: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn cache_returns_by_id_and_evicts_oldest() {
        let state = AppState::new();
        let first = state.insert_gradebook(book("a"));
        assert_eq!(state.gradebook(&first).unwrap().source, PathBuf::from("a"));
        for i in 0..CACHE_CAP {
            state.insert_gradebook(book(&format!("b{i}")));
        }
        let err = state.gradebook(&first).unwrap_err();
        assert_eq!(err.code, AppErrorCode::Internal);
        assert!(state.gradebook("gb-nope").is_err());
    }

    #[test]
    fn one_generation_at_a_time_and_cancel_flag() {
        let state = AppState::new();
        assert!(!state.request_cancel(), "nothing running");
        let guard = state.begin_generation().unwrap();
        assert!(state.begin_generation().is_err());
        assert!(!guard.cancel_flag().load(Ordering::SeqCst));
        assert!(state.request_cancel());
        assert!(guard.cancel_flag().load(Ordering::SeqCst));
        drop(guard);
        let again = state.begin_generation().unwrap();
        assert!(
            !again.cancel_flag().load(Ordering::SeqCst),
            "flag reset for a new run"
        );
    }

    #[test]
    fn roots_are_deduplicated() {
        let state = AppState::new();
        state.allow_root(Path::new("/a"));
        state.allow_root(Path::new("/a"));
        state.allow_root(Path::new("/b"));
        assert_eq!(state.session_roots().len(), 2);
    }
}
