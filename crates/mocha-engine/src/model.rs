//! Data models. Mirrors the dataclasses in `legacy/src/libs/excel_analyzer.py`
//! (lines 35-88). Field names, order and meaning are identical; maps keep
//! insertion order like Python `dict`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Metadata extracted from the gradebook header. Mirrors `FileMetadata`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_name: String,
    pub university_name: String,
    pub program_name: String,
    pub materia_name: String,
    pub group_name: String,
    pub professor_name: String,
    pub period_name: String,
    pub first_twenty_grade_count: usize,
    pub first_twenty_grade_numbers: Vec<String>,
    pub component_grade_numbers: Vec<String>,
    pub grade_labels: IndexMap<String, String>,
    /// Python default: `"20%"`.
    pub first_block_weight_label: String,
    /// Python default: empty dict.
    pub component_weight_labels: IndexMap<String, String>,
}

/// Grades of one student. Mirrors `StudentGrades`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentGrades {
    pub student_number: Option<i64>,
    pub student_code: String,
    pub student_name: String,
    pub abse: String,
    pub first_twenty_grades: IndexMap<String, String>,
    pub component_grades: IndexMap<String, String>,
    pub weighted_components: IndexMap<String, String>,
    pub definitive_grade: String,
}

/// The normalized cell grid the parser works on (checkpoint A): what the
/// Python parser sees after `pd.read_csv(header=None, dtype=str,
/// keep_default_na=False)`. Rectangular; every cell is text, empty = `""`.
pub type Grid = Vec<Vec<String>>;
