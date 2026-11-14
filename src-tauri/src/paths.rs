//! Folder naming, allowed-path checks and output-folder inspection.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::error::AppError;

/// Longest folder-name part kept, in characters. Windows paths are limited to
/// about 260 characters in total, so names stay well below that.
const MAX_PART_CHARS: usize = 80;
/// Separator of the suggested folder name (an en dash with spaces).
const SEPARATOR: &str = " \u{2013} ";
/// Folder name used when nothing usable remains.
const FALLBACK_NAME: &str = "Boletines";

const RESERVED_WINDOWS_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Makes `raw` safe as one folder-name component on Windows and Linux.
///
/// Accents and other letters are kept. `<>:"/\|?*` and control characters
/// become `-`; whitespace runs collapse to one space; leading and trailing
/// dots and spaces are removed (Windows refuses trailing ones); reserved
/// Windows device names (`CON`, `NUL`, `COM1`, ...) get a `_` prefix; the
/// result is cut at 80 characters. Returns an empty string when nothing
/// usable remains.
pub fn sanitize_name(raw: &str) -> String {
    let replaced: String = raw
        .chars()
        .map(|c| {
            if c.is_whitespace() {
                ' '
            } else if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut name: String = collapsed.trim_matches(|c| c == '.' || c == ' ').to_string();
    if name.chars().count() > MAX_PART_CHARS {
        name = name.chars().take(MAX_PART_CHARS).collect();
        name = name.trim_matches(|c| c == '.' || c == ' ').to_string();
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    if RESERVED_WINDOWS_NAMES.contains(&stem.as_str()) {
        name.insert(0, '_');
    }
    name
}

/// Builds `<Materia> – Grupo <n> – <Periodo>`, sanitized. Empty parts are
/// left out; a group that already starts with "Grupo" is not prefixed again.
/// With no usable part at all the result is `Boletines`.
pub fn suggested_folder_name(subject: &str, group: &str, period: &str) -> String {
    let group = group.trim();
    let group_part = if group.is_empty() {
        String::new()
    } else if group.to_lowercase().starts_with("grupo") {
        group.to_string()
    } else {
        format!("Grupo {group}")
    };
    let parts: Vec<String> = [subject.trim(), group_part.as_str(), period.trim()]
        .iter()
        .map(|p| sanitize_name(p))
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        FALLBACK_NAME.to_string()
    } else {
        parts.join(SEPARATOR)
    }
}

/// Canonicalizes `path` (resolving `..` and symlinks) and checks that it lies
/// inside one of `roots` (each canonicalized too; roots that do not exist are
/// skipped). Returns the canonical path.
///
/// # Errors
///
/// `FILE_NOT_FOUND` when `path` does not exist; `INTERNAL` when it is
/// outside every root.
pub fn resolve_allowed(path: &Path, roots: &[PathBuf]) -> Result<PathBuf, AppError> {
    let canonical = fs::canonicalize(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::file_not_found(path)
        } else {
            AppError::path_not_allowed(path)
        }
    })?;
    let inside = roots
        .iter()
        .filter_map(|r| fs::canonicalize(r).ok())
        .any(|root| canonical.starts_with(&root));
    if inside {
        Ok(canonical)
    } else {
        Err(AppError::path_not_allowed(path))
    }
}

/// Like [`resolve_allowed`], and the file must be a `.pdf` file.
///
/// # Errors
///
/// As [`resolve_allowed`]; `INTERNAL` when it is not a PDF file.
pub fn resolve_allowed_pdf(path: &Path, roots: &[PathBuf]) -> Result<PathBuf, AppError> {
    let canonical = resolve_allowed(path, roots)?;
    let is_pdf = canonical.is_file()
        && canonical
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    if is_pdf {
        Ok(canonical)
    } else {
        Err(AppError::path_not_allowed(path))
    }
}

/// What the UI needs to know about a candidate output folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct OutputDirStatus {
    /// The path exists (as a folder or as something else).
    pub exists: bool,
    /// Reports can be written there (or, if it does not exist yet, in the
    /// nearest existing parent so that it can be created).
    pub writable: bool,
    /// Number of `*.pdf` files directly inside it.
    pub pdf_count: u32,
    /// A free sibling path of the form `<name> (2)`, `(3)`, ...
    pub alternative_path: String,
    /// The last component of `alternative_path`.
    pub alternative_name: String,
}

fn probe_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".mocha-write-test-{}", std::process::id()));
    match fs::File::create(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn count_pdfs(dir: &Path) -> u32 {
    let Ok(read) = fs::read_dir(dir) else {
        return 0;
    };
    let n = read
        .filter_map(Result::ok)
        .filter(|e| {
            let p = e.path();
            p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf"))
        })
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn free_alternative(path: &Path) -> PathBuf {
    let name = path.file_name().map_or_else(
        || FALLBACK_NAME.to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut n = 2u32;
    loop {
        let candidate = parent.join(format!("{name} ({n})"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Inspects a candidate output folder (plan 6.2 overwrite choice).
pub fn check_output_dir(path: &Path) -> OutputDirStatus {
    let exists = path.exists();
    let writable = if exists {
        path.is_dir() && probe_writable(path)
    } else {
        path.ancestors()
            .skip(1)
            .find(|a| a.is_dir())
            .is_some_and(probe_writable)
    };
    let pdf_count = if exists { count_pdfs(path) } else { 0 };
    let alt = free_alternative(path);
    let alternative_name = alt
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    OutputDirStatus {
        exists,
        writable,
        pdf_count,
        alternative_path: alt.to_string_lossy().into_owned(),
        alternative_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppErrorCode;

    #[test]
    fn keeps_accents_and_replaces_forbidden_characters() {
        assert_eq!(sanitize_name("FRANÇAIS IV"), "FRANÇAIS IV");
        assert_eq!(
            sanitize_name("Ramírez: <Año> \"2\" a/b\\c|d?e*f"),
            "Ramírez- -Año- -2- a-b-c-d-e-f"
        );
        assert_eq!(sanitize_name("tab\there\nnew"), "tab here new");
        assert_eq!(sanitize_name("a\u{0}b"), "a-b");
    }

    #[test]
    fn trims_trailing_dots_and_spaces() {
        assert_eq!(sanitize_name("  hola. . "), "hola");
        assert_eq!(sanitize_name("..oculto"), "oculto");
        assert_eq!(sanitize_name("..."), "");
        assert_eq!(sanitize_name("   "), "");
        assert_eq!(sanitize_name(""), "");
    }

    #[test]
    fn reserved_windows_names_are_prefixed() {
        assert_eq!(sanitize_name("CON"), "_CON");
        assert_eq!(sanitize_name("nul"), "_nul");
        assert_eq!(sanitize_name("Com1"), "_Com1");
        assert_eq!(sanitize_name("lpt9.txt"), "_lpt9.txt");
        assert_eq!(sanitize_name("CONSOLA"), "CONSOLA");
        assert_eq!(sanitize_name("COM10"), "COM10");
    }

    #[test]
    fn long_names_are_cut_on_a_character_boundary() {
        let long = "é".repeat(200);
        let out = sanitize_name(&long);
        assert_eq!(out.chars().count(), MAX_PART_CHARS);
        let spaced = format!("{} .", "a".repeat(MAX_PART_CHARS - 1));
        assert_eq!(sanitize_name(&spaced), "a".repeat(MAX_PART_CHARS - 1));
    }

    #[test]
    fn suggested_name_follows_the_plan() {
        assert_eq!(
            suggested_folder_name("FRANÇAIS IV", "02", "2025"),
            "FRANÇAIS IV \u{2013} Grupo 02 \u{2013} 2025"
        );
        assert_eq!(
            suggested_folder_name("FRANÇAIS IV", "Grupo 2", "2025-2"),
            "FRANÇAIS IV \u{2013} Grupo 2 \u{2013} 2025-2"
        );
    }

    #[test]
    fn suggested_name_handles_empty_and_hostile_parts() {
        assert_eq!(suggested_folder_name("", "02", ""), "Grupo 02");
        assert_eq!(
            suggested_folder_name("Lengua", "", "2025"),
            "Lengua \u{2013} 2025"
        );
        assert_eq!(suggested_folder_name("  ", "", ""), "Boletines");
        assert_eq!(
            suggested_folder_name("A/B", "1.", "CON"),
            "A-B \u{2013} Grupo 1 \u{2013} _CON"
        );
        assert_eq!(suggested_folder_name("...", "...", "..."), "Grupo");
    }

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(root.join("a.pdf"), b"%PDF").unwrap();
        fs::write(root.join("a.txt"), b"x").unwrap();
        fs::write(outside.join("secret.pdf"), b"%PDF").unwrap();
        (tmp, root, outside)
    }

    #[test]
    fn allows_paths_inside_roots_only() {
        let (_tmp, root, outside) = setup();
        let roots = vec![root.clone()];
        assert!(resolve_allowed_pdf(&root.join("a.pdf"), &roots).is_ok());
        let err = resolve_allowed_pdf(&outside.join("secret.pdf"), &roots).unwrap_err();
        assert_eq!(err.code, AppErrorCode::Internal);
        // The root itself is allowed as a folder but is not a PDF.
        assert!(resolve_allowed(&root, &roots).is_ok());
        assert!(resolve_allowed_pdf(&root, &roots).is_err());
        // Not a PDF.
        assert!(resolve_allowed_pdf(&root.join("a.txt"), &roots).is_err());
        // Missing files report FILE_NOT_FOUND.
        let missing = resolve_allowed_pdf(&root.join("nope.pdf"), &roots).unwrap_err();
        assert_eq!(missing.code, AppErrorCode::FileNotFound);
        // No roots, nothing allowed.
        assert!(resolve_allowed_pdf(&root.join("a.pdf"), &[]).is_err());
        // A sibling folder sharing a name prefix is not inside the root.
        let sibling = root.with_file_name("root2");
        fs::create_dir_all(&sibling).unwrap();
        fs::write(sibling.join("b.pdf"), b"%PDF").unwrap();
        assert!(resolve_allowed_pdf(&sibling.join("b.pdf"), &roots).is_err());
    }

    #[test]
    fn rejects_traversal() {
        let (_tmp, root, outside) = setup();
        let roots = vec![root.clone()];
        let sneaky = root.join("..").join("outside").join("secret.pdf");
        assert!(sneaky.exists());
        assert!(resolve_allowed_pdf(&sneaky, &roots).is_err());
        // Traversal that stays inside is fine and canonicalized.
        let inner = root.join("..").join("root").join("a.pdf");
        let ok = resolve_allowed_pdf(&inner, &roots).unwrap();
        assert_eq!(ok, fs::canonicalize(root.join("a.pdf")).unwrap());
        let _ = outside;
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_pointing_outside() {
        use std::os::unix::fs::symlink;
        let (_tmp, root, outside) = setup();
        let roots = vec![root.clone()];
        symlink(outside.join("secret.pdf"), root.join("link.pdf")).unwrap();
        assert!(resolve_allowed_pdf(&root.join("link.pdf"), &roots).is_err());
        symlink(&outside, root.join("linkdir")).unwrap();
        assert!(resolve_allowed_pdf(&root.join("linkdir").join("secret.pdf"), &roots).is_err());
        // A root reached through a symlink still matches by canonical path.
        let alias = root.with_file_name("alias");
        symlink(&root, &alias).unwrap();
        assert!(resolve_allowed_pdf(&alias.join("a.pdf"), &roots).is_ok());
    }

    #[test]
    fn output_dir_status_counts_pdfs_and_proposes_alternative() {
        let (_tmp, root, _outside) = setup();
        fs::write(root.join("B.PDF"), b"x").unwrap();
        fs::create_dir_all(root.join("sub.pdf")).unwrap();
        let s = check_output_dir(&root);
        assert!(s.exists && s.writable);
        assert_eq!(
            s.pdf_count, 2,
            "counts *.pdf files only, any case, not folders"
        );
        assert_eq!(s.alternative_name, "root (2)");
        assert!(!Path::new(&s.alternative_path).exists());
        fs::create_dir_all(&s.alternative_path).unwrap();
        assert_eq!(check_output_dir(&root).alternative_name, "root (3)");
    }

    #[test]
    fn output_dir_status_for_missing_and_file_paths() {
        let (_tmp, root, _outside) = setup();
        let missing = root.join("new").join("deeper");
        let s = check_output_dir(&missing);
        assert!(!s.exists);
        assert!(
            s.writable,
            "can be created under an existing writable parent"
        );
        assert_eq!(s.pdf_count, 0);
        assert_eq!(s.alternative_name, "deeper (2)");
        let file = check_output_dir(&root.join("a.txt"));
        assert!(file.exists);
        assert!(!file.writable);
        assert_eq!(file.pdf_count, 0);
    }

    #[cfg(unix)]
    #[test]
    fn read_only_folder_is_not_writable() {
        use std::os::unix::fs::PermissionsExt;
        let (_tmp, root, _outside) = setup();
        let ro = root.join("ro");
        fs::create_dir_all(&ro).unwrap();
        fs::set_permissions(&ro, fs::Permissions::from_mode(0o555)).unwrap();
        let s = check_output_dir(&ro);
        let running_as_root = fs::File::create(ro.join("probe")).is_ok();
        if !running_as_root {
            assert!(!s.writable);
        }
        fs::set_permissions(&ro, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
