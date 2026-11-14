//! Tabular layouts (2019 / 2025): `GradebookParser._find_header_row` and the
//! tabular branch of `GradebookParser.parse` (py lines 648-874).

use indexmap::IndexMap;

use crate::model::{FileMetadata, Grid, StudentGrades};
use crate::patterns::{DECIMAL_TOKEN, DIGITS_ONLY};
use crate::patterns::{GROUP_ROW_PATTERN, PERIOD_ROW_PATTERN, PROFESSOR_ROW_PATTERN};
use crate::pycompat::py_int_from_float_str;
use crate::text::{
    clean_text, extract_group, extract_period, extract_professor, format_weight_label, path_name,
    to_grade_str,
};

use super::helpers::{
    cell, cell_or_empty, clean_lower_row, clean_row, find_header_col, find_header_prefix_col,
    find_name_col, first_matching_in_row, first_non_empty_in_row, py_int_str, row_cells, NO_NAMES,
};
use super::labels::extract_grade_labels;
use super::{ParseError, ParseOutput};

/// Port of `_find_header_row` (py 862-874): first row whose lower-cased
/// cleaned cells contain the number, code, name and `abse` headers. `None`
/// stands for the `ValueError` that `parse` catches to fall back to the
/// carnet layout.
pub fn find_header_row(grid: &Grid) -> Option<usize> {
    for row_idx in 0..grid.len() {
        let row = clean_lower_row(row_cells(grid, row_idx));
        if find_header_col(&row, NO_NAMES, None).is_some()
            && find_header_col(&row, &["code"], None).is_some()
            && find_name_col(&row, None).is_some()
            && find_header_prefix_col(&row, &["abse"], None).is_some()
        {
            return Some(row_idx);
        }
    }
    None
}

/// `str(int(float(token)))` for a header token that matched
/// `\d+(?:\.\d+)?`; Python raises `OverflowError` when the float is `inf`.
fn grade_num_from_token(token: &str) -> Result<String, ParseError> {
    py_int_from_float_str(token).ok_or_else(|| ParseError::InvalidGradeNumber {
        cell: token.to_string(),
    })
}

/// `parse_student_number` closure (py 765-769): the spreadsheet number as
/// the canonical decimal text of `int(number)`, `None` when the cell is not
/// all digits. (Kept as text because Python integers are unbounded.)
fn parse_student_number(
    row: &[String],
    student_no_col: Option<usize>,
) -> Result<Option<String>, ParseError> {
    let number = clean_text(student_no_col.map_or("", |c| cell_or_empty(row, c)));
    if DIGITS_ONLY.is_match(&number) {
        return py_int_str(&number).map(Some);
    }
    Ok(None)
}

/// The tabular branch of `GradebookParser.parse` (py 672-860), entered once
/// [`find_header_row`] found `header_row_idx`. Returns the metadata, the
/// students and the `Student enumeration corrected...` warning if any.
pub fn parse_tabular(
    grid: &Grid,
    csv_file: &str,
    header_row_idx: usize,
) -> Result<ParseOutput, ParseError> {
    let weights_row_idx = header_row_idx.saturating_sub(1); // max(0, h - 1)

    let header = clean_row(row_cells(grid, header_row_idx));
    let weights = clean_row(row_cells(grid, weights_row_idx));

    let student_no_col = find_header_col(&header, NO_NAMES, Some(0));
    let student_code_col = find_header_col(&header, &["code"], Some(1));
    let student_name_col = find_name_col(&header, Some(2));
    // Python would raise a TypeError on `None + 1`; unreachable because the
    // header row was selected by the same finders.
    let abse_col = find_header_prefix_col(&header, &["abse"], Some(3)).unwrap_or(3);
    let first_grade_col = abse_col + 1;

    // py 683-689: the 'moy' column (end of the first-block grades).
    let moy_col = match weights
        .iter()
        .position(|token| token.to_lowercase() == "moy")
    {
        Some(i) => i,
        None => {
            let mut c = first_grade_col;
            while c < header.len() && !clean_text(&header[c]).is_empty() {
                c += 1;
            }
            c
        }
    };

    // py 691-694. `header[col]` cannot be out of range: `moy_col <= len`.
    let first_twenty_cols: Vec<usize> = (first_grade_col..moy_col)
        .filter(|&col| {
            header
                .get(col)
                .is_some_and(|h| DECIMAL_TOKEN.is_match(&clean_text(h)))
        })
        .collect();

    // py 696-701
    let mut first_twenty_grade_numbers: Vec<String> = Vec::new();
    for &col in &first_twenty_cols {
        let token = clean_text(&header[col]);
        if !token.is_empty() {
            first_twenty_grade_numbers.push(grade_num_from_token(&token)?);
        }
    }

    // py 703-710
    let mut component_grade_cols: Vec<usize> = Vec::new();
    let mut component_grade_numbers: Vec<String> = Vec::new();
    for (i, cell_text) in header.iter().enumerate().skip(moy_col + 1) {
        let token = clean_text(cell_text);
        if DECIMAL_TOKEN.is_match(&token) {
            component_grade_cols.push(i);
            component_grade_numbers.push(grade_num_from_token(&token)?);
        }
    }

    // py 712-723
    let weighted_first_col = moy_col + 1;
    let first_block_weight_label = format_weight_label(
        weights.get(weighted_first_col).map_or("", String::as_str),
        "20%",
    );

    let mut component_weight_labels: IndexMap<String, String> = IndexMap::new();
    for (&col, grade_num) in component_grade_cols.iter().zip(&component_grade_numbers) {
        let weighted_col = col + 1;
        component_weight_labels.insert(
            grade_num.clone(),
            format_weight_label(
                weights.get(weighted_col).map_or("", String::as_str),
                &first_block_weight_label,
            ),
        );
    }

    // py 725-730: first header starting with "défin" / "defin".
    let definitive_col = header.iter().position(|token| {
        let lower = token.to_lowercase();
        lower.starts_with("d\u{e9}fin") || lower.starts_with("defin")
    });

    let grade_labels = extract_grade_labels(grid);

    // py 735-755
    let metadata = FileMetadata {
        file_name: path_name(csv_file).to_string(),
        university_name: first_non_empty_in_row(grid, 0),
        program_name: first_non_empty_in_row(grid, 1),
        materia_name: first_non_empty_in_row(grid, 3),
        group_name: extract_group(&first_matching_in_row(grid, 4, GROUP_ROW_PATTERN.as_ref())),
        professor_name: extract_professor(&first_matching_in_row(
            grid,
            5,
            PROFESSOR_ROW_PATTERN.as_ref(),
        )),
        period_name: extract_period(&first_matching_in_row(grid, 6, PERIOD_ROW_PATTERN.as_ref())),
        first_twenty_grade_count: first_twenty_grade_numbers.len(),
        first_twenty_grade_numbers: first_twenty_grade_numbers.clone(),
        component_grade_numbers: component_grade_numbers.clone(),
        grade_labels: grade_labels.clone(),
        first_block_weight_label: first_block_weight_label.clone(),
        component_weight_labels: component_weight_labels.clone(),
    };

    // py 758-777: student rows are those with a name and (a number or a code).
    let mut student_rows: Vec<&[String]> = Vec::new();
    for row in grid.iter().skip(header_row_idx + 1) {
        let number = parse_student_number(row, student_no_col)?;
        let code = clean_text(student_code_col.map_or("", |c| cell_or_empty(row, c)));
        let name = clean_text(student_name_col.map_or("", |c| cell_or_empty(row, c)));
        if !name.is_empty() && (number.is_some() || !code.is_empty()) {
            student_rows.push(row.as_slice());
        }
    }

    let mut students: Vec<StudentGrades> = Vec::new();
    let mut enumeration_fallbacks: Vec<String> = Vec::new();

    for (idx, row) in student_rows.iter().enumerate() {
        let row: &[String] = row;
        let expected_student_number = idx + 1;
        let spreadsheet_student_number = parse_student_number(row, student_no_col)?;
        if spreadsheet_student_number.as_deref()
            != Some(expected_student_number.to_string().as_str())
        {
            let student_name = clean_text(student_name_col.map_or("", |c| cell_or_empty(row, c)));
            let observed = spreadsheet_student_number
                .as_deref()
                .unwrap_or("blank/non-numeric");
            let shown = if student_name.is_empty() {
                "unnamed student"
            } else {
                student_name.as_str()
            };
            enumeration_fallbacks.push(format!("{shown}: {observed} -> {expected_student_number}"));
        }

        // py 798-803: first-block grades.
        let mut first_twenty_grades: IndexMap<String, String> = IndexMap::new();
        for &col in &first_twenty_cols {
            let grade_num = grade_num_from_token(&clean_text(&header[col]))?;
            let label = grade_labels
                .get(&grade_num)
                .cloned()
                .unwrap_or_else(|| format!("Grade {grade_num}"));
            first_twenty_grades.insert(
                format!("{grade_num} - {label}"),
                to_grade_str(cell(row, col)?),
            );
        }

        // py 805-824: averages, weighted values and component grades.
        let mut component_grades: IndexMap<String, String> = IndexMap::new();
        let mut weighted_components: IndexMap<String, String> = IndexMap::new();

        let first_block_average = if moy_col < row.len() {
            to_grade_str(cell(row, moy_col)?)
        } else {
            String::new()
        };
        let first_block_weighted = if weighted_first_col < row.len() {
            to_grade_str(cell(row, weighted_first_col)?)
        } else {
            String::new()
        };
        component_grades.insert(
            format!("1-{} average", first_twenty_grade_numbers.len()),
            first_block_average,
        );
        weighted_components.insert(
            format!(
                "1-{} weighted {}",
                first_twenty_grade_numbers.len(),
                first_block_weight_label
            ),
            first_block_weighted,
        );

        for &col in &component_grade_cols {
            let grade_num = grade_num_from_token(&clean_text(&header[col]))?;
            let label = grade_labels
                .get(&grade_num)
                .cloned()
                .unwrap_or_else(|| format!("Grade {grade_num}"));
            component_grades.insert(
                format!("{grade_num} - {label}"),
                to_grade_str(cell(row, col)?),
            );

            let weighted_col = col + 1;
            if weighted_col < row.len() {
                let weight_label = component_weight_labels
                    .get(&grade_num)
                    .map_or(first_block_weight_label.as_str(), String::as_str);
                weighted_components.insert(
                    format!("{grade_num} weighted {weight_label}"),
                    to_grade_str(cell(row, weighted_col)?),
                );
            }
        }

        // py 826
        let definitive_grade = match definitive_col {
            Some(c) => to_grade_str(cell(row, c)?),
            None => String::new(),
        };

        students.push(StudentGrades {
            student_number: i64::try_from(expected_student_number).ok(),
            student_code: clean_text(student_code_col.map_or("", |c| cell_or_empty(row, c))),
            student_name: clean_text(student_name_col.map_or("", |c| cell_or_empty(row, c))),
            abse: to_grade_str(cell_or_empty(row, abse_col)),
            first_twenty_grades,
            component_grades,
            weighted_components,
            definitive_grade,
        });
    }

    // py 840-845
    let mut warnings = Vec::new();
    if !enumeration_fallbacks.is_empty() {
        let mut preview = enumeration_fallbacks
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        let extra = enumeration_fallbacks.len().saturating_sub(5);
        if extra > 0 {
            preview = format!("{preview}; and {extra} more");
        }
        warnings.push(format!(
            "Student enumeration corrected from row order: {preview}"
        ));
    }

    Ok(ParseOutput {
        metadata,
        students,
        warnings,
    })
}
