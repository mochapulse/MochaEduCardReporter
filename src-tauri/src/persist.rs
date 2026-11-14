//! Key-value persistence behind a small trait, so settings, recent files and
//! history can be tested without a running app. The production implementation
//! wraps the Tauri store plugin.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

/// A minimal persistent key-value store of JSON values.
pub trait KeyValueStore: Send + Sync {
    /// Reads a value.
    fn get(&self, key: &str) -> Option<Value>;
    /// Writes a value and makes it durable.
    ///
    /// # Errors
    ///
    /// A description of the failure.
    fn set(&self, key: &str, value: Value) -> Result<(), String>;
}

/// In-memory store for tests.
#[derive(Debug, Default)]
pub struct MemoryStore {
    data: Mutex<HashMap<String, Value>>,
}

impl MemoryStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyValueStore for MemoryStore {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.lock().ok()?.get(key).cloned()
    }

    fn set(&self, key: &str, value: Value) -> Result<(), String> {
        self.data
            .lock()
            .map_err(|e| e.to_string())?
            .insert(key.to_string(), value);
        Ok(())
    }
}

/// Store backed by the `tauri-plugin-store` file `state.json`.
pub struct TauriStore<R: tauri::Runtime> {
    store: Arc<tauri_plugin_store::Store<R>>,
}

impl<R: tauri::Runtime> TauriStore<R> {
    /// Opens (or creates) the store file in the app data folder.
    ///
    /// # Errors
    ///
    /// The plugin's error, as text.
    pub fn open(app: &impl tauri_plugin_store::StoreExt<R>) -> Result<Self, String> {
        let store = app.store("state.json").map_err(|e| e.to_string())?;
        Ok(Self { store })
    }
}

impl<R: tauri::Runtime> KeyValueStore for TauriStore<R> {
    fn get(&self, key: &str) -> Option<Value> {
        self.store.get(key)
    }

    fn set(&self, key: &str, value: Value) -> Result<(), String> {
        self.store.set(key.to_string(), value);
        self.store.save().map_err(|e| e.to_string())
    }
}

/// Reads `key` as `T`, falling back to `T::default()` when the value is
/// missing or no longer matches the expected shape (a corrupt or older file
/// must never block the app).
pub fn load_or_default<T: serde::de::DeserializeOwned + Default>(
    store: &dyn KeyValueStore,
    key: &str,
) -> T {
    match store.get(key) {
        None => T::default(),
        Some(v) => serde_json::from_value(v).unwrap_or_else(|e| {
            log::warn!("ignoring unreadable stored value for '{key}': {e}");
            T::default()
        }),
    }
}

/// Serializes `value` under `key`.
///
/// # Errors
///
/// A description of the failure.
pub fn save<T: serde::Serialize>(
    store: &dyn KeyValueStore,
    key: &str,
    value: &T,
) -> Result<(), String> {
    let v = serde_json::to_value(value).map_err(|e| e.to_string())?;
    store.set(key, v)
}
