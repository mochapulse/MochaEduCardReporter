//! Carnet layout: per-student blocks exported from report-card templates
//! (`GradebookParser._parse_carnet_blocks` and helpers, py lines 876-1064).

use std::collections::HashSet;

use indexmap::IndexMap;

use crate::model::{FileMetadata, Grid, StudentGrades};
use crate::patterns::DIGITS_ONLY;
use crate::text::{
    clean_text, extract_absences, extract_period, extract_sheet_label_from_csv_name,
    format_weight_label, is_component_label, is_final_carnet_row, path_name, path_stem,
    to_grade_str,
};

use super::helpers::{
    cell, cell_or_empty, clean_lower_row, clean_row, find_header_col, first_non_empty_in_row,
    first_numeric_header_col, py_int_str, row_cells, row_is_blank, NO_NAMES,
};
use super::{ParseError, ParseOutput};

/// Port of `_find_carnet_header_rows` (py 1035-1045): every row that holds
/// the `no`, `item`, `note` and `moy` headers.
pub fn find_carnet_header_rows(grid: &Grid) -> Vec<usize> {
    let mut header_rows = Vec::new();
    for row_idx in 0..grid.len() {
        let row = clean_lower_row(row_cells(grid, row_idx));
        let has_number = find_header_col(&row, NO_NAMES, None).is_some();
        let has_item = find_header_col(&row, &["item"], None).is_some();
        let has_note = find_header_col(&row, &["note"], None).is_some();
        let has_moy = find_header_col(&row, &["moy"], None).is_some();
        if has_number && has_item && has_note && has_moy {
            header_rows.push(row_idx);
        }
    }
    header_rows
}

/// Port of `_collect_carnet_items` (py 1047-1064): `(grade number, label)`
/// pairs of the rows under a header, stopping at the `note du cours` row or
/// at the first blank row that is not an item.
pub fn collect_carnet_items(
    grid: &Grid,
    header_row_idx: usize,
    no_col: usize,
    item_col: usize,
) -> Result<Vec<(String, String)>, ParseError> {
    let mut items = Vec::new();
    for row_idx in (header_row_idx + 1)..grid.len() {
        let row = row_cells(grid, row_idx);
        let item_text = clean_text(cell_or_empty(row, item_col));
        if is_final_carnet_row(&item_text) {
            break;
        }
        let grade_num = clean_text(cell_or_empty(row, no_col));
        if DIGITS_ONLY.is_match(&grade_num) && !item_text.is_empty() {
            items.push((py_int_str(&grade_num)?, item_text));
        } else if row_is_blank(row) {
            break;
        }
    }
    Ok(items)
}

/// Port of `_parse_carnet_blocks` (py 876-954): metadata from the first
/// header block, then one student per header row.
pub fn parse_carnet_blocks(grid: &Grid, csv_file: &str) -> Result<ParseOutput, ParseError> {
    let header_rows = find_carnet_header_rows(grid);
    let Some(&first_header) = header_rows.first() else {
        return Err(ParseError::NoSupportedLayout);
    };

    let header = clean_row(row_cells(grid, first_header));
    let no_col = find_header_col(&header, NO_NAMES, Some(0)).unwrap_or(0);
    let item_col = find_header_col(&header, &["item"], Some(1)).unwrap_or(1);
    let note_col = find_header_col(&header, &["note"], Some(2)).unwrap_or(2);
    let moy_col = find_header_col(&header, &["moy"], Some(3)).unwrap_or(3);
    // py 888: `_first_numeric_header_col(...) or 4` (0 is falsy as well).
    let weighted_col = first_numeric_header_col(&header, Some(moy_col))
        .filter(|&c| c != 0)
        .unwrap_or(4);
    // py 889: `header[weighted_col]` raises IndexError past the row end.
    let first_block_weight_label = format_weight_label(cell(&header, weighted_col)?, "20%");

    let first_items = collect_carnet_items(grid, first_header, no_col, item_col)?;
    let first_block_numbers: Vec<String> = first_items
        .iter()
        .filter(|(_, label)| !is_component_label(label))
        .map(|(n, _)| n.clone())
        .collect();
    let component_numbers: Vec<String> = first_items
        .iter()
        .filter(|(_, label)| is_component_label(label))
        .map(|(n, _)| n.clone())
        .collect();
    let mut grade_labels: IndexMap<String, String> = IndexMap::new();
    for (grade_num, label) in &first_items {
        grade_labels.insert(grade_num.clone(), label.clone());
    }
    let mut component_weight_labels: IndexMap<String, String> = IndexMap::new();
    for grade_num in &component_numbers {
        component_weight_labels.insert(grade_num.clone(), first_block_weight_label.clone());
    }

    let subject = first_non_empty_in_row(grid, first_header.saturating_sub(3));
    let period = extract_period(path_stem(csv_file));
    let sheet_label = extract_sheet_label_from_csv_name(csv_file);

    let metadata = FileMetadata {
        file_name: path_name(csv_file).to_string(),
        university_name: String::new(),
        program_name: String::new(),
        materia_name: subject,
        group_name: sheet_label,
        professor_name: String::new(),
        period_name: period,
        first_twenty_grade_count: first_block_numbers.len(),
        first_twenty_grade_numbers: first_block_numbers.clone(),
        component_grade_numbers: component_numbers.clone(),
        grade_labels: grade_labels.clone(),
        first_block_weight_label: first_block_weight_label.clone(),
        component_weight_labels: component_weight_labels.clone(),
    };

    let mut students = Vec::new();
    for (idx, &header_row_idx) in header_rows.iter().enumerate() {
        students.push(parse_carnet_student(
            grid,
            idx + 1,
            header_row_idx,
            &CarnetColumns {
                no_col,
                item_col,
                note_col,
                moy_col,
                weighted_col,
            },
            &CarnetContext {
                first_block_numbers: &first_block_numbers,
                component_numbers: &component_numbers,
                grade_labels: &grade_labels,
                first_block_weight_label: &first_block_weight_label,
                component_weight_labels: &component_weight_labels,
            },
        )?);
    }

    Ok(ParseOutput {
        metadata,
        students,
        warnings: Vec::new(),
    })
}

/// Column positions of a carnet block (the parameters `no_col` ... `weighted_col`
/// of `_parse_carnet_student`).
pub struct CarnetColumns {
    pub no_col: usize,
    pub item_col: usize,
    pub note_col: usize,
    pub moy_col: usize,
    pub weighted_col: usize,
}

/// The block-independent data of `_parse_carnet_student` (its list/dict
/// parameters).
pub struct CarnetContext<'a> {
    pub first_block_numbers: &'a [String],
    pub component_numbers: &'a [String],
    pub grade_labels: &'a IndexMap<String, String>,
    pub first_block_weight_label: &'a str,
    pub component_weight_labels: &'a IndexMap<String, String>,
}

/// Port of `_parse_carnet_student` (py 956-1033): reads the block under
/// `header_row_idx` until a blank row or the `note du cours` row. The block
/// does NOT stop at the next header row; only at those two markers.
pub fn parse_carnet_student(
    grid: &Grid,
    student_number: usize,
    header_row_idx: usize,
    cols: &CarnetColumns,
    ctx: &CarnetContext<'_>,
) -> Result<StudentGrades, ParseError> {
    let student_name = first_non_empty_in_row(grid, header_row_idx.saturating_sub(2));
    let abse = extract_absences(&first_non_empty_in_row(
        grid,
        header_row_idx.saturating_sub(1),
    ));

    let first_block_set: HashSet<&str> =
        ctx.first_block_numbers.iter().map(String::as_str).collect();
    let component_set: HashSet<&str> = ctx.component_numbers.iter().map(String::as_str).collect();
    let mut first_twenty_grades: IndexMap<String, String> = IndexMap::new();
    let mut component_grades: IndexMap<String, String> = IndexMap::new();
    let mut weighted_components: IndexMap<String, String> = IndexMap::new();
    let mut first_block_average = String::new();
    let mut first_block_weighted = String::new();
    let mut definitive_grade = String::new();

    let opt = |row: &[String], col: usize| -> String {
        row.get(col).map_or_else(String::new, |v| to_grade_str(v))
    };

    for row_idx in (header_row_idx + 1)..grid.len() {
        let row = row_cells(grid, row_idx);
        let item_text = clean_text(cell_or_empty(row, cols.item_col));
        if item_text.is_empty() {
            if row_is_blank(row) {
                break;
            }
            continue;
        }

        if is_final_carnet_row(&item_text) {
            definitive_grade = opt(row, cols.weighted_col);
            break;
        }

        let grade_num = clean_text(cell_or_empty(row, cols.no_col));
        if !DIGITS_ONLY.is_match(&grade_num) {
            continue;
        }

        let grade_num = py_int_str(&grade_num)?;
        let label = ctx
            .grade_labels
            .get(&grade_num)
            .map_or(item_text.as_str(), String::as_str);

        if first_block_set.contains(grade_num.as_str()) {
            first_twenty_grades.insert(format!("{grade_num} - {label}"), opt(row, cols.note_col));
            if first_block_average.is_empty() && cols.moy_col < row.len() {
                first_block_average = opt(row, cols.moy_col);
            }
            if first_block_weighted.is_empty() && cols.weighted_col < row.len() {
                first_block_weighted = opt(row, cols.weighted_col);
            }
        } else if component_set.contains(grade_num.as_str()) {
            component_grades.insert(format!("{grade_num} - {label}"), opt(row, cols.moy_col));
            let weight_label = ctx
                .component_weight_labels
                .get(&grade_num)
                .map_or(ctx.first_block_weight_label, String::as_str);
            weighted_components.insert(
                format!("{grade_num} weighted {weight_label}"),
                opt(row, cols.weighted_col),
            );
        }
    }

    component_grades.insert(
        format!("1-{} average", ctx.first_block_numbers.len()),
        first_block_average,
    );
    weighted_components.insert(
        format!(
            "1-{} weighted {}",
            ctx.first_block_numbers.len(),
            ctx.first_block_weight_label
        ),
        first_block_weighted,
    );

    Ok(StudentGrades {
        student_number: i64::try_from(student_number).ok(),
        student_code: String::new(),
        student_name,
        abse,
        first_twenty_grades,
        component_grades,
        weighted_components,
        definitive_grade,
    })
}
