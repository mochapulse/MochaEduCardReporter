//! Text helpers ported from `legacy/src/libs/excel_analyzer.py`: the module
//! level utility functions (py lines 93-275) and the string-only static
//! helpers of `GradebookParser` (py lines 1074-1177).
//!
//! In the Python engine every cell reaches these functions as `str` (the grid
//! is read with `dtype=str, keep_default_na=False`), so the `pd.isna(...)`
//! and `isinstance(value, (float, int))` branches are unreachable for string
//! input; only the string path is mirrored here.

use crate::patterns::{
    ABSENCES, DATE_START, DOT_BEFORE_UPPER, FILENAME_UNSAFE, FILENAME_WS, FIRST_DIGITS,
    GRADE_KEY_NUM, PROFESSOR_PREFIX, SHEET_LABEL, WS_RUN, YEAR,
};
use crate::pycompat::{fmt_1f, fmt_g, py_float, py_strip};

/// Port of `clean_text` (py 93-109): NBSP to space, `strip()`, then collapse
/// every whitespace run (Python `\s`) to a single space.
pub fn clean_text(value: &str) -> String {
    let text = value.replace('\u{A0}', " ");
    WS_RUN.replace_all(py_strip(&text), " ").into_owned()
}

/// Port of `to_grade_str` (py 112-134): a numeric text becomes `float` with
/// one decimal (`"15"` gives `"15.0"`); non-numeric text is returned cleaned.
pub fn to_grade_str(value: &str) -> String {
    let text = clean_text(value);
    if !text.is_empty() {
        if let Some(f) = py_float(&text) {
            return fmt_1f(f);
        }
    }
    text
}

/// Port of `to_float` (py 137-154): cleaned text through Python `float()`,
/// `None` for empty or non-numeric text (`nan`/`inf` are `Some`).
pub fn to_float(value: &str) -> Option<f64> {
    let text = clean_text(value);
    if text.is_empty() {
        return None;
    }
    py_float(&text)
}

/// Port of `extract_grade_num_from_key` (py 157-169): the leading digits of a
/// `"123 - Label"` key (`re.match(r"^(\d+)\s*-")`).
pub fn extract_grade_num_from_key(key: &str) -> Option<String> {
    GRADE_KEY_NUM
        .captures(key)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Port of `format_concept_label` (py 172-212): split into category,
/// description and date lines joined with `<br/>`.
pub fn format_concept_label(label: &str) -> String {
    // py 185: add a missing space after a period before an uppercase letter.
    let label = DOT_BEFORE_UPPER.replace_all(label, ". ${1}").into_owned();

    let dot_idx = label.find(". ");
    let colon_idx = label.find(": ");

    let (category, rest) = match (dot_idx, colon_idx) {
        (Some(d), c) if c.is_none_or(|c| d < c) => (
            format!("{}.", py_strip(&label[..d])),
            py_strip(&label[d + 2..]),
        ),
        (_, Some(c)) => (
            format!("{}:", py_strip(&label[..c])),
            py_strip(&label[c + 2..]),
        ),
        _ => return label,
    };

    if rest.is_empty() {
        return category;
    }

    if let Some(m) = DATE_START.find(rest) {
        let desc = py_strip(&rest[..m.start()]);
        let date = py_strip(&rest[m.start()..]);
        if !desc.is_empty() && !date.is_empty() {
            return format!("{category}<br/>{desc}<br/>{date}");
        }
        if !date.is_empty() {
            return format!("{category}<br/>{date}");
        }
        return format!("{category}<br/>{desc}");
    }

    format!("{category}<br/>{rest}")
}

/// Port of `extract_label_from_key` (py 215-228): the part after the first
/// `" - "` (stripped), or the whole key, run through [`format_concept_label`].
pub fn extract_label_from_key(key: &str) -> String {
    let label = match key.split_once(" - ") {
        Some((_, rest)) => py_strip(rest),
        None => key,
    };
    format_concept_label(label)
}

/// Port of `safe_filename` (py 231-244): drop characters other than word
/// characters, `-` and whitespace, strip, whitespace runs to `_`; `"student"`
/// if the result is empty.
pub fn safe_filename(value: &str) -> String {
    let removed = FILENAME_UNSAFE.replace_all(value, "");
    let cleaned = FILENAME_WS
        .replace_all(py_strip(&removed), "_")
        .into_owned();
    if cleaned.is_empty() {
        "student".to_string()
    } else {
        cleaned
    }
}

/// Port of `format_grade` (py 247-258): the grade text as-is (`value if value
/// else ""`).
pub fn format_grade(value: &str) -> String {
    value.to_string()
}

/// Port of `format_weight_label` (py 261-275): a weight cell as a percentage
/// label (`"0,2"` gives `"20%"`, `"15%"` gives `"15%"`); `default` when the
/// cell is empty, the cleaned text when it is not numeric.
pub fn format_weight_label(value: &str, default: &str) -> String {
    let text = clean_text(value);
    if text.is_empty() {
        return default.to_string();
    }

    let has_percent = text.contains('%');
    let replaced = text.replace('%', "").replace(',', ".");
    let normalized = py_strip(&replaced);
    let Some(number) = py_float(normalized) else {
        return text;
    };

    let percent = if has_percent || number > 1.0 {
        number
    } else {
        number * 100.0
    };
    format!("{}%", fmt_g(percent))
}

/// Port of `GradebookParser._extract_group` (py 1158-1163): first run of
/// digits, or `""`.
pub fn extract_group(value: &str) -> String {
    let text = clean_text(value);
    FIRST_DIGITS
        .captures(&text)
        .and_then(|c| c.get(1))
        .map_or_else(String::new, |m| m.as_str().to_string())
}

/// Port of `GradebookParser._extract_professor` (py 1165-1169): the cleaned
/// text without a leading `PROFESSEUR :` prefix (case-insensitive).
pub fn extract_professor(value: &str) -> String {
    let text = clean_text(value);
    PROFESSOR_PREFIX.replace(&text, "").into_owned()
}

/// Port of `GradebookParser._extract_period` (py 1171-1176): the first four
/// consecutive digits, else the cleaned text.
pub fn extract_period(value: &str) -> String {
    let text = clean_text(value);
    match YEAR.captures(&text).and_then(|c| c.get(1)) {
        Some(m) => m.as_str().to_string(),
        None => text,
    }
}

/// Port of `GradebookParser._extract_absences` (py 1082-1087): the number
/// after `Absences :` formatted with [`to_grade_str`] (comma turned into a
/// point), or `""`.
pub fn extract_absences(value: &str) -> String {
    let text = clean_text(value);
    match ABSENCES.captures(&text).and_then(|c| c.get(1)) {
        Some(m) => to_grade_str(&m.as_str().replace(',', ".")),
        None => String::new(),
    }
}

/// Python `Path(p).name` on POSIX (`/` separator; `\` also on Windows):
/// last component after dropping empty and `.` components.
pub fn path_name(path: &str) -> &str {
    let is_sep = |c: char| c == '/' || (cfg!(windows) && c == '\\');
    path.split(is_sep)
        .rfind(|p| !p.is_empty() && *p != ".")
        .unwrap_or("")
}

/// Python `Path(p).stem`: the name without its last suffix (a dot that is
/// neither the first nor the last character of the name).
pub fn path_stem(path: &str) -> &str {
    let name = path_name(path);
    // `PurePath.stem`: split at the last dot unless it is the first or last char.
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => &name[..i],
        _ => name,
    }
}

/// Port of `GradebookParser._extract_sheet_label_from_csv_name` (py
/// 1093-1097): the part of `Path(csv_file).stem` from `_FRAN...` on (the
/// match is case-insensitive), underscores turned into spaces; `""` if absent.
pub fn extract_sheet_label_from_csv_name(csv_file: &str) -> String {
    let stem = path_stem(csv_file);
    match SHEET_LABEL.captures(stem).and_then(|c| c.get(1)) {
        Some(m) => m.as_str().replace('_', " "),
        None => String::new(),
    }
}

/// Port of `GradebookParser._is_component_label` (py 1074-1076): the cleaned,
/// lower-cased label contains `"examen"`.
pub fn is_component_label(label: &str) -> bool {
    clean_text(label).to_lowercase().contains("examen")
}

/// Port of `GradebookParser._is_final_carnet_row` (py 1078-1080): the cleaned,
/// lower-cased cell starts with `"note du cours"`.
pub fn is_final_carnet_row(value: &str) -> bool {
    clean_text(value)
        .to_lowercase()
        .starts_with("note du cours")
}

/// Port of `GradebookParser._normalize_header_token` (py 1099-1101): cleaned,
/// lower-cased, stripped.
pub fn normalize_header_token(value: &str) -> String {
    py_strip(&clean_text(value).to_lowercase()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_and_filename() {
        assert_eq!(clean_text("a\u{1C}b\u{1D} c\u{A0}\u{A0}d "), "a b c d");
        assert_eq!(safe_filename("\u{B2}"), "\u{B2}");
        assert_eq!(safe_filename("x\u{B2}y"), "x\u{B2}y");
        assert_eq!(safe_filename("a\u{301}b"), "ab");
        assert_eq!(safe_filename("a\u{1C}b"), "a_b");
        assert_eq!(safe_filename("\u{663}x"), "\u{663}x");
        assert_eq!(safe_filename("a\u{203F}b"), "ab");
        assert_eq!(safe_filename("a\u{200D}b"), "ab");
        assert_eq!(safe_filename("a_b-c d"), "a_b-c_d");
        assert_eq!(safe_filename("!!!"), "student");
    }

    #[test]
    fn grades_and_weights() {
        assert_eq!(to_grade_str("15"), "15.0");
        assert_eq!(to_grade_str(" abc "), "abc");
        assert_eq!(to_grade_str(""), "");
        assert_eq!(to_float("4,5"), None);
        assert_eq!(to_float("1_0"), Some(10.0));
        assert_eq!(format_weight_label("0,2", "20%"), "20%");
        assert_eq!(format_weight_label("15%", "20%"), "15%");
        assert_eq!(format_weight_label("1", "20%"), "100%");
        assert_eq!(format_weight_label("", "30%"), "30%");
        assert_eq!(format_weight_label("abc", "20%"), "abc");
        assert_eq!(format_weight_label("0.125", "20%"), "12.5%");
    }

    #[test]
    fn labels() {
        assert_eq!(
            format_concept_label("Oral.Le 5 mars"),
            "Oral.<br/>Le 5 mars"
        );
        assert_eq!(
            format_concept_label("Exam: Foo Le 12 mai"),
            "Exam:<br/>Foo<br/>Le 12 mai"
        );
        assert_eq!(extract_label_from_key("12 - Foo. Bar"), "Foo.<br/>Bar");
        assert_eq!(extract_grade_num_from_key("12 - Foo"), Some("12".into()));
        assert_eq!(extract_grade_num_from_key("12Foo"), None);
    }

    #[test]
    fn parser_helpers() {
        assert_eq!(extract_absences("Absences : 3,5"), "3.5");
        assert_eq!(extract_absences("ABSENCES:2"), "2.0");
        assert_eq!(extract_absences("ABs\u{17F}"), "");
        assert_eq!(extract_absences("AB\u{17F}ENCES:2"), "2.0");
        assert_eq!(extract_group("Groupe 03"), "03");
        assert_eq!(extract_professor("Professeur : Jean"), "Jean");
        assert_eq!(extract_period("Periode 2025-2"), "2025");
        assert_eq!(extract_period("none"), "none");
        assert_eq!(
            extract_sheet_label_from_csv_name("/tmp/x/a_Francais_IV.csv"),
            "Francais IV"
        );
        assert_eq!(extract_sheet_label_from_csv_name("a_FRAN.csv"), "");
        assert_eq!(
            extract_sheet_label_from_csv_name("a_fran_x.tar.csv"),
            "fran x.tar"
        );
        assert!(is_component_label(" EXAMEN 1"));
        assert!(is_final_carnet_row("Note du cours"));
        assert_eq!(normalize_header_token(" \u{A0}NOM  "), "nom");
    }
}
