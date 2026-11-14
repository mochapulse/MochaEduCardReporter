//! Port of `GradebookParser._extract_grade_labels` (py lines 618-646).

use indexmap::IndexMap;

use crate::model::Grid;
use crate::patterns::{GRADE_LABEL_CELL, HAS_LETTER};
use crate::pycompat::py_strip;
use crate::text::clean_text;

/// Port of `_extract_grade_labels` (py 618-646): scans the WHOLE grid in row
/// then column order; every cleaned cell matching
/// `^(\d+)\s*[\.,]*\s+(.+)$` whose label contains a letter sets
/// `labels[digits]`. The last match for a number wins but keeps the position
/// of the first one (dict re-assignment); the number text is kept as written
/// (`"05"` stays `"05"`).
pub fn extract_grade_labels(grid: &Grid) -> IndexMap<String, String> {
    let mut labels: IndexMap<String, String> = IndexMap::new();
    let ncols = grid.first().map_or(0, Vec::len);

    for row in grid {
        for col_idx in 0..ncols {
            let cell = clean_text(row.get(col_idx).map_or("", String::as_str));
            if cell.is_empty() {
                continue;
            }
            if let Some(caps) = GRADE_LABEL_CELL.captures(&cell) {
                let (Some(num), Some(label)) = (caps.get(1), caps.get(2)) else {
                    continue;
                };
                let label = py_strip(label.as_str());
                if HAS_LETTER.is_match(label) {
                    labels.insert(num.as_str().to_string(), label.to_string());
                }
            }
        }
    }

    labels
}
