//! Runs the Python-generated vectors in `fixtures/expected/pycompat.json`
//! through the Rust text helpers and the Python-semantics layer.

use mocha_engine::pycompat::{fmt_1f, fmt_g, py_float};
use mocha_engine::text::*;
use serde_json::Value;

const VECTORS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/expected/pycompat.json"
);

/// Expected value of a vector.
enum Out {
    Text(String),
    Null,
}

/// Result of a Rust function, comparable with [`Out`].
enum Got {
    Text(String),
    Null,
    Float(Option<f64>),
}

fn fmt_got(g: &Got) -> String {
    match g {
        Got::Text(s) => format!("{s:?}"),
        Got::Null => "null".to_string(),
        Got::Float(Some(f)) => format!("float {f:?}"),
        Got::Float(None) => "null".to_string(),
    }
}

fn same_float(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
}

/// Compare numerically (distinguishing -0.0 from 0.0, nan == nan).
fn matches(out: &Out, got: &Got) -> bool {
    match (out, got) {
        (Out::Null, Got::Null) | (Out::Null, Got::Float(None)) => true,
        (Out::Text(e), Got::Text(g)) => e == g,
        (Out::Text(e), Got::Float(Some(g))) => e.parse::<f64>().is_ok_and(|e| same_float(e, *g)),
        _ => false,
    }
}

fn run(name: &str, input: &str) -> Option<Got> {
    let t = |s: String| Some(Got::Text(s));
    let b = |v: bool| Some(Got::Text(v.to_string()));
    match name {
        "clean_text" => t(clean_text(input)),
        "to_grade_str" => t(to_grade_str(input)),
        "to_float" => Some(Got::Float(to_float(input))),
        "extract_grade_num_from_key" => {
            Some(extract_grade_num_from_key(input).map_or(Got::Null, Got::Text))
        }
        "format_concept_label" => t(format_concept_label(input)),
        "extract_label_from_key" => t(extract_label_from_key(input)),
        "safe_filename" => t(safe_filename(input)),
        "format_weight_label" => t(format_weight_label(input, "20%")),
        "extract_group" => t(extract_group(input)),
        "extract_professor" => t(extract_professor(input)),
        "extract_period" => t(extract_period(input)),
        "extract_absences" => t(extract_absences(input)),
        "extract_sheet_label_from_csv_name" => t(extract_sheet_label_from_csv_name(input)),
        "is_component_label" => b(is_component_label(input)),
        "is_final_carnet_row" => b(is_final_carnet_row(input)),
        "py_float" => Some(Got::Float(py_float(input))),
        "fmt_1f" => py_float(input).and_then(|x| t(fmt_1f(x))),
        "fmt_g" => py_float(input).and_then(|x| t(fmt_g(x))),
        _ => None,
    }
}

#[test]
fn python_vectors_match() {
    let raw =
        std::fs::read_to_string(VECTORS).unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    let doc: Value = serde_json::from_str(&raw).expect("invalid JSON");
    let map = doc.as_object().expect("top level must be an object");

    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    for (name, vectors) in map {
        let vectors = vectors.as_array().expect("vector list must be an array");
        for v in vectors {
            total += 1;
            let input = v["in"].as_str().expect("`in` must be a string");
            let out = match &v["out"] {
                Value::String(s) => Out::Text(s.clone()),
                Value::Null => Out::Null,
                other => panic!("bad `out` {other:?}"),
            };
            match run(name, input) {
                None => {
                    failures.push(format!(
                        "{name}: no Rust counterpart or unparsable input {input:?}"
                    ));
                    break;
                }
                Some(got) if !matches(&out, &got) => {
                    let exp = match &out {
                        Out::Text(s) => format!("{s:?}"),
                        Out::Null => "null".to_string(),
                    };
                    failures.push(format!(
                        "{name}({input:?}): expected {exp}, got {}",
                        fmt_got(&got)
                    ));
                }
                Some(_) => {}
            }
        }
    }
    assert!(total > 0, "no vectors found");
    assert!(
        failures.is_empty(),
        "{} of {total} vectors failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("{total} vectors passed");
}
