//! User settings (plan section 6.5) and the legacy-workspace migration.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;
use crate::persist::{load_or_default, save, KeyValueStore};

const KEY: &str = "settings";

/// Interface text size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum TextSize {
    /// `normal`
    #[default]
    Normal,
    /// `grande`
    Grande,
    /// `muy-grande`
    MuyGrande,
}

/// Interface theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    /// `claro`: light theme (the default, plan section 5).
    #[default]
    Claro,
    /// `oscuro`
    Oscuro,
    /// `sistema`: follow the operating system.
    Sistema,
}

/// The persisted settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Settings {
    /// Base folder for report cards; `None` = use the default.
    pub base_folder: Option<String>,
    /// Whether the chosen base folder is remembered.
    pub remember_base_folder: bool,
    /// Interface text size.
    pub text_size: TextSize,
    /// Interface theme.
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            base_folder: None,
            remember_base_folder: true,
            text_size: TextSize::Normal,
            theme: Theme::Claro,
        }
    }
}

impl Settings {
    /// The stored base folder, if set and not blank.
    pub fn base_folder_value(&self) -> Option<&str> {
        self.base_folder
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}

/// Loads the settings (defaults when nothing valid is stored).
pub fn load(store: &dyn KeyValueStore) -> Settings {
    load_or_default(store, KEY)
}

/// Saves the settings. A blank base folder is stored as `None`.
///
/// # Errors
///
/// `INTERNAL` when the store cannot be written.
pub fn store(store: &dyn KeyValueStore, settings: &Settings) -> Result<(), AppError> {
    let mut s = settings.clone();
    s.base_folder = s.base_folder_value().map(str::to_string);
    save(store, KEY, &s).map_err(|e| AppError::internal(format!("cannot save settings: {e}")))
}

/// Reads the legacy workspace path (`~/.mocha_edu_card_reporter/workspace_path.txt`).
///
/// Returns `None` when the file is missing, unreadable or blank.
pub fn read_legacy_workspace(home: &Path) -> Option<PathBuf> {
    let file = home
        .join(".mocha_edu_card_reporter")
        .join("workspace_path.txt");
    let text = std::fs::read_to_string(file).ok()?;
    let line = text.trim().trim_start_matches('\u{feff}').trim();
    (!line.is_empty()).then(|| PathBuf::from(line))
}

/// The default base folder: the stored setting, else the legacy workspace
/// (plan section 7, migration), else `<Documents>/Boletines`.
pub fn default_output_base(
    settings: &Settings,
    home: Option<&Path>,
    documents: Option<&Path>,
) -> PathBuf {
    if let Some(base) = settings.base_folder_value() {
        return PathBuf::from(base);
    }
    if let Some(legacy) = home.and_then(read_legacy_workspace) {
        return legacy;
    }
    documents
        .or(home)
        .map(|d| d.join("Boletines"))
        .unwrap_or_else(|| PathBuf::from("Boletines"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::MemoryStore;

    #[test]
    fn defaults_when_empty_and_round_trip() {
        let mem = MemoryStore::new();
        assert_eq!(load(&mem), Settings::default());
        let s = Settings {
            base_folder: Some("  /tmp/x ".into()),
            remember_base_folder: false,
            text_size: TextSize::MuyGrande,
            theme: Theme::Oscuro,
        };
        store(&mem, &s).unwrap();
        let back = load(&mem);
        assert_eq!(back.base_folder.as_deref(), Some("/tmp/x"));
        assert_eq!(back.text_size, TextSize::MuyGrande);
        assert!(!back.remember_base_folder);
    }

    #[test]
    fn serialized_values_are_the_ui_strings() {
        let v = serde_json::to_value(Settings {
            text_size: TextSize::MuyGrande,
            theme: Theme::Sistema,
            ..Settings::default()
        })
        .unwrap();
        assert_eq!(v["text_size"], "muy-grande");
        assert_eq!(v["theme"], "sistema");
        assert_eq!(serde_json::to_value(TextSize::Grande).unwrap(), "grande");
        assert_eq!(serde_json::to_value(Theme::Claro).unwrap(), "claro");
    }

    #[test]
    fn corrupt_or_partial_values_fall_back() {
        let mem = MemoryStore::new();
        mem.set(KEY, serde_json::json!({"theme": "neon"})).unwrap();
        assert_eq!(load(&mem), Settings::default());
        mem.set(KEY, serde_json::json!({"theme": "oscuro"}))
            .unwrap();
        let s = load(&mem);
        assert_eq!(s.theme, Theme::Oscuro);
        assert!(s.remember_base_folder);
    }

    #[test]
    fn blank_base_folder_is_none() {
        let mem = MemoryStore::new();
        store(
            &mem,
            &Settings {
                base_folder: Some("   ".into()),
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(load(&mem).base_folder, None);
    }

    #[test]
    fn default_base_prefers_setting_then_legacy_then_documents() {
        let home = tempfile::tempdir().unwrap();
        let docs = home.path().join("Documents");
        let none = Settings::default();
        assert_eq!(
            default_output_base(&none, Some(home.path()), Some(&docs)),
            docs.join("Boletines")
        );

        let dir = home.path().join(".mocha_edu_card_reporter");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("workspace_path.txt"), "\u{feff}/old/workspace\r\n").unwrap();
        assert_eq!(
            default_output_base(&none, Some(home.path()), Some(&docs)),
            PathBuf::from("/old/workspace")
        );

        let set = Settings {
            base_folder: Some("/chosen".into()),
            ..Settings::default()
        };
        assert_eq!(
            default_output_base(&set, Some(home.path()), Some(&docs)),
            PathBuf::from("/chosen")
        );
    }

    #[test]
    fn blank_legacy_file_is_ignored() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join(".mocha_edu_card_reporter");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("workspace_path.txt"), "  \n").unwrap();
        assert_eq!(read_legacy_workspace(home.path()), None);
        assert_eq!(
            default_output_base(&Settings::default(), Some(home.path()), None),
            home.path().join("Boletines")
        );
    }
}
