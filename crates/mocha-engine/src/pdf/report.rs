//! Port of `PDFGenerator` (`legacy/src/libs/excel_analyzer.py` lines 1181-1598):
//! story, grade table, rows, styles and table styles for one student report.

use std::path::{Path, PathBuf};

use crate::patterns::ISO_TIME_TAIL;

use super::document::{a4, build, mm, Flowable, PageSetup};
use super::fonts::Face;
use super::paragraph::{Align, ParaStyle, Paragraph};
use super::table::{Cell, Cmd, HAlign, LineOp, Table, VAlign};
use super::writer::{write_pdf, Color, DocInfo};
use super::PdfError;
use crate::model::{FileMetadata, StudentGrades};
use crate::text::{
    extract_grade_num_from_key, extract_label_from_key, format_concept_label, format_grade,
    safe_filename,
};

/// Row index metadata of `_build_student_rows` (`row_meta` dict). Indices are
/// 0-based over the data rows (header excluded); `first_block_end` is `-1` when
/// there are no first-block rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowMeta {
    /// `first_block_start`
    pub first_block_start: i64,
    /// `first_block_end`
    pub first_block_end: i64,
    /// `component_start`
    pub component_start: i64,
    /// `component_end`
    pub component_end: i64,
    /// `final_row`
    pub final_row: i64,
}

/// The paragraph styles of `_create_styles`, with ReportLab's inheritance resolved
/// (`Title`: leading 22, spaceAfter overridden to 4; `Normal`: leading 12).
struct Styles {
    title: ParaStyle,
    header: ParaStyle,
    name: ParaStyle,
    cell: ParaStyle,
    cell_first_block: ParaStyle,
    cell_component: ParaStyle,
    cell_final: ParaStyle,
}

const PURPLE: u32 = 0x7A2CB8;
const GREEN: u32 = 0x3F5F23;

/// Mirrors `_create_styles`.
fn create_styles() -> Styles {
    let normal = ParaStyle::normal();
    let cell = ParaStyle {
        font_size: 7.0,
        leading: 9.0,
        ..normal.clone()
    };
    Styles {
        title: ParaStyle {
            font: Face::HelveticaBold,
            font_size: 12.0,
            leading: 22.0,
            alignment: Align::Center,
            space_after: 4.0,
            ..normal.clone()
        },
        header: ParaStyle {
            font_size: 8.0,
            leading: 10.0,
            ..normal.clone()
        },
        name: ParaStyle {
            font: Face::HelveticaBold,
            font_size: 9.0,
            space_after: 2.0,
            ..normal
        },
        cell_first_block: ParaStyle {
            text_color: Color::hex(PURPLE),
            ..cell.clone()
        },
        cell_component: ParaStyle {
            text_color: Color::hex(GREEN),
            ..cell.clone()
        },
        cell_final: ParaStyle {
            font: Face::HelveticaBold,
            text_color: Color::RED,
            ..cell.clone()
        },
        cell,
    }
}

/// Mirrors `PDFGenerator._build_student_rows` (py 1421-1499).
pub fn build_student_rows(
    metadata: &FileMetadata,
    student: &StudentGrades,
) -> (Vec<[String; 4]>, RowMeta) {
    let mut rows: Vec<[String; 4]> = Vec::new();

    let mut ordered_first: Vec<(&String, &String)> = student.first_twenty_grades.iter().collect();
    ordered_first.sort_by_key(|(k, _)| {
        extract_grade_num_from_key(k)
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0)
    });

    let first_block_start = 0usize;
    let weight_label = &metadata.first_block_weight_label;
    for (key, grade) in ordered_first {
        let concept = extract_label_from_key(key);
        rows.push([
            concept,
            format_grade(grade),
            weight_label.clone(),
            String::new(),
        ]);
    }

    let weighted_avg_key = format!(
        "1-{} weighted {}",
        metadata.first_twenty_grade_count, weight_label
    );
    let weighted_avg_value = student.weighted_components.get(&weighted_avg_key);

    let first_block_end = rows.len() as i64 - 1;
    if !rows.is_empty() {
        rows[first_block_start][3] = format_grade(weighted_avg_value.map_or("", String::as_str));
    }

    let component_start = rows.len() as i64;
    for grade_num in &metadata.component_grade_numbers {
        let label = metadata
            .grade_labels
            .get(grade_num)
            .cloned()
            .unwrap_or_else(|| format!("NOTE {grade_num}"));
        let concept = format_concept_label(&label);

        let prefix = format!("{grade_num} -");
        let component_value = student
            .component_grades
            .iter()
            .find(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v.as_str());

        let weight = metadata
            .component_weight_labels
            .get(grade_num)
            .unwrap_or(weight_label)
            .clone();
        let weighted_key = format!("{grade_num} weighted {weight}");
        let weighted_value = student.weighted_components.get(&weighted_key);

        rows.push([
            concept,
            format_grade(component_value.unwrap_or("")),
            weight,
            format_grade(weighted_value.map_or("", String::as_str)),
        ]);
    }

    let final_row = rows.len() as i64;
    rows.push([
        "NOTE DÉFINITIVE".to_string(),
        String::new(),
        String::new(),
        format_grade(&student.definitive_grade),
    ]);

    let meta = RowMeta {
        first_block_start: first_block_start as i64,
        first_block_end,
        component_start,
        component_end: final_row - 1,
        final_row,
    };
    (rows, meta)
}

/// Mirrors `_style_for_grade_row`.
fn style_for_grade_row(row_idx: i64, m: &RowMeta, s: &Styles) -> ParaStyle {
    if row_idx == m.final_row {
        s.cell_final.clone()
    } else if m.first_block_start <= row_idx && row_idx <= m.first_block_end {
        s.cell_first_block.clone()
    } else if m.component_start <= row_idx && row_idx <= m.component_end {
        s.cell_component.clone()
    } else {
        s.cell.clone()
    }
}

/// Mirrors `_create_table_styles` (py 1550-1598).
fn create_table_styles(m: &RowMeta) -> Vec<Cmd> {
    let first_start = (1 + m.first_block_start) as i32;
    let first_end = (1 + m.first_block_end) as i32;
    let comp_start = (1 + m.component_start) as i32;
    let comp_end = (1 + m.component_end) as i32;
    let final_row = (1 + m.final_row) as i32;
    let black = Color::BLACK;
    vec![
        Cmd::Background(((0, 0), (-1, 0)), Color::LIGHTGREY),
        Cmd::TextColor(((0, 0), (-1, 0)), black),
        Cmd::FontName(((0, 0), (-1, 0)), Face::HelveticaBold),
        Cmd::FontSize(((0, 0), (-1, 0)), 7.0),
        Cmd::Align(((0, 0), (0, -1)), HAlign::Center),
        Cmd::Align(((2, 0), (-1, -1)), HAlign::Center),
        Cmd::Align(((1, 0), (1, -1)), HAlign::Left),
        Cmd::VAlign(((0, 0), (-1, -1)), VAlign::Middle),
        Cmd::Line(LineOp::Grid, ((0, 0), (-1, -1)), 0.5, black),
        Cmd::LeftPadding(((0, 0), (-1, -1)), 2.0),
        Cmd::RightPadding(((0, 0), (-1, -1)), 2.0),
        Cmd::TopPadding(((0, 0), (-1, -1)), 1.0),
        Cmd::BottomPadding(((0, 0), (-1, -1)), 1.0),
        Cmd::TextColor(((0, first_start), (-1, first_end)), Color::hex(PURPLE)),
        Cmd::TextColor(((0, comp_start), (-1, comp_end)), Color::hex(GREEN)),
        Cmd::Line(
            LineOp::LineAbove,
            ((0, comp_start), (-1, comp_start)),
            1.2,
            black,
        ),
        Cmd::Span(((3, first_start), (3, first_end))),
        Cmd::Span(((4, first_start), (4, first_end))),
        Cmd::Align(((3, first_start), (4, first_end)), HAlign::Center),
        Cmd::Span(((0, final_row), (3, final_row))),
        Cmd::Align(((0, final_row), (3, final_row)), HAlign::Right),
        Cmd::TextColor(((0, final_row), (-1, final_row)), Color::RED),
        Cmd::FontName(((0, final_row), (-1, final_row)), Face::HelveticaBold),
        Cmd::FontSize(((0, final_row), (-1, final_row)), 9.0),
        Cmd::Line(
            LineOp::LineAbove,
            ((0, final_row), (-1, final_row)),
            1.0,
            black,
        ),
    ]
}

/// Mirrors `_build_grade_table`.
fn build_grade_table(
    metadata: &FileMetadata,
    student: &StudentGrades,
    styles: &Styles,
) -> Result<Table, PdfError> {
    let (raw_rows, meta) = build_student_rows(metadata, student);
    let mut data: Vec<Vec<Cell>> = vec![["No", "CONCEPT", "NOTE", "%", "MOY"]
        .iter()
        .map(|s| Cell::Text((*s).to_string()))
        .collect()];
    let mut row_number = 1;
    for (row_idx, [concept, note, pct, moy]) in raw_rows.iter().enumerate() {
        let style = style_for_grade_row(row_idx as i64, &meta, styles);
        let is_final = row_idx as i64 == meta.final_row;
        let first = if is_final {
            Paragraph::new("", &style)?
        } else {
            Paragraph::new(&row_number.to_string(), &style)?
        };
        data.push(vec![
            Cell::Para(first),
            Cell::Para(Paragraph::new(concept, &style)?),
            Cell::Para(Paragraph::new(note, &style)?),
            Cell::Para(Paragraph::new(pct, &style)?),
            Cell::Para(Paragraph::new(moy, &style)?),
        ]);
        if !is_final {
            row_number += 1;
        }
    }
    let m = mm();
    let mut table = Table::new(
        data,
        vec![10.0 * m, 104.0 * m, 18.0 * m, 16.0 * m, 18.0 * m],
        1,
    );
    table.set_style(&create_table_styles(&meta));
    Ok(table)
}

/// Python `datetime.fromisoformat(ts.replace('Z', '+00:00')).strftime("%d/%m/%Y")`,
/// for the ISO forms the app produces (`YYYY-MM-DD` optionally followed by a
/// separator character, a time and a UTC offset). `None` when Python would raise.
fn format_date(ts: &str) -> Option<String> {
    let s = ts.replace('Z', "+00:00");
    let date = s.get(..10)?;
    let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let rest = &s[10..];
    if !rest.is_empty() {
        let mut chars = rest.chars();
        chars.next()?;
        let time = chars.as_str();
        let re = ISO_TIME_TAIL.as_ref()?;
        if !re.is_match(time) {
            return None;
        }
        let hh: u32 = time[..2].parse().ok()?;
        if hh > 23 {
            return None;
        }
    }
    Some(d.format("%d/%m/%Y").to_string())
}

/// Mirrors `_build_document_story`.
fn build_document_story(
    metadata: &FileMetadata,
    student: &StudentGrades,
    timestamp_iso: Option<&str>,
    styles: &Styles,
) -> Result<Vec<Flowable>, PdfError> {
    let mut story = Vec::new();
    let one_mm = mm();
    let para = |t: &str, s: &ParaStyle| Paragraph::new(t, s).map(Flowable::Para);

    if let Some(ts) = timestamp_iso.filter(|t| !t.is_empty()) {
        let text = match format_date(ts) {
            Some(d) => format!("Date: {d}"),
            None => format!("Date: {ts}"),
        };
        story.push(para(&text, &styles.header)?);
        story.push(Flowable::Spacer(one_mm));
    }

    story.push(para("RÉSULTAT FINAL", &styles.title)?);
    story.push(para(&metadata.university_name, &styles.header)?);
    story.push(para(&metadata.program_name, &styles.header)?);
    story.push(para(&metadata.materia_name, &styles.header)?);
    story.push(para(
        &format!("GROUPE {} - {}", metadata.group_name, metadata.period_name),
        &styles.header,
    )?);
    story.push(para(
        &format!("PROFESSEUR : {}", metadata.professor_name),
        &styles.header,
    )?);
    story.push(Flowable::Spacer(one_mm));

    story.push(para(&student.student_name.to_uppercase(), &styles.name)?);
    story.push(para(
        &format!("Code étudiant : {}", student.student_code),
        &styles.header,
    )?);
    story.push(para(
        &format!("Absences : {}", format_grade(&student.abse)),
        &styles.header,
    )?);
    story.push(Flowable::Spacer(one_mm));

    story.push(Flowable::Table(Box::new(build_grade_table(
        metadata, student, styles,
    )?)));
    story.push(Flowable::Spacer(one_mm));

    // ParagraphStyle("Foot", parent=header, fontSize=8): identical to header.
    story.push(para("Document généré automatiquement.", &styles.header)?);
    Ok(story)
}

/// Same as [`generate_student_pdf`], returning the PDF bytes.
pub fn render_student_pdf(
    metadata: &FileMetadata,
    student: &StudentGrades,
    timestamp_iso: Option<&str>,
) -> Result<Vec<u8>, PdfError> {
    let styles = create_styles();
    let story = build_document_story(metadata, student, timestamp_iso, &styles)?;
    let (w, h) = a4();
    let m = mm();
    let setup = PageSetup {
        width: w,
        height: h,
        left: 10.0 * m,
        right: 10.0 * m,
        top: 8.0 * m,
        bottom: 8.0 * m,
    };
    let pages = build(&setup, story)?;
    Ok(write_pdf(
        &pages,
        w,
        h,
        &DocInfo {
            title: "RÉSULTAT FINAL",
            author: &metadata.professor_name,
        },
    ))
}

/// Mirrors `PDFGenerator._generate_student_pdf`: writes
/// `<output_dir>/<NN>_<safe name>.pdf` and returns its path.
pub fn generate_student_pdf(
    metadata: &FileMetadata,
    student: &StudentGrades,
    output_dir: &Path,
    timestamp_iso: Option<&str>,
) -> Result<PathBuf, PdfError> {
    let student_num = student.student_number.unwrap_or(0);
    let pdf_name = format!(
        "{:02}_{}.pdf",
        student_num,
        safe_filename(&student.student_name)
    );
    let path = output_dir.join(pdf_name);
    let bytes = render_student_pdf(metadata, student, timestamp_iso)?;
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(&path, bytes)?;
    Ok(path)
}
