//! Checkpoint A: the reader's grid equals the Python oracle's grid, cell by
//! cell, for the fixture corpus (`fixtures/expected`) and the synthetic reader
//! corpus (`fixtures/synthetic-reader/expected`).

use std::fs;
use std::path::{Path, PathBuf};

use mocha_engine::reader::{excel_sheet_csv_text, get_sheet_names, read_grid};
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn grid_of(v: &Value) -> Vec<Vec<String>> {
    v.as_array()
        .expect("grid is an array")
        .iter()
        .map(|r| {
            r.as_array()
                .expect("row is an array")
                .iter()
                .map(|c| c.as_str().expect("cell is a string").to_string())
                .collect()
        })
        .collect()
}

fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("names array")
        .iter()
        .map(|s| s.as_str().expect("name").to_string())
        .collect()
}

fn diff_grids(expected: &[Vec<String>], got: &[Vec<String>], out: &mut Vec<String>, id: &str) {
    if expected.len() != got.len() {
        out.push(format!(
            "{id}: row count expected {} got {}",
            expected.len(),
            got.len()
        ));
    }
    let mut count = 0;
    let mut shown = 0;
    for (r, (e, g)) in expected.iter().zip(got).enumerate() {
        if e.len() != g.len() {
            out.push(format!(
                "{id}: row {r} width expected {} got {}",
                e.len(),
                g.len()
            ));
        }
        for (c, (ec, gc)) in e.iter().zip(g).enumerate() {
            if ec != gc {
                count += 1;
                if shown < 5 {
                    shown += 1;
                    out.push(format!("{id}: cell ({r},{c}) expected {ec:?} got {gc:?}"));
                }
            }
        }
    }
    if count > 0 {
        out.push(format!("{id}: {count} differing cells"));
    }
}

fn run_corpus(expected_dir: &Path, corpus_dir: &Path) -> (usize, Vec<String>) {
    let manifest = json(&expected_dir.join("manifest.json"));
    let mut problems = Vec::new();
    let mut checked = 0;
    for f in manifest["files"].as_array().expect("files") {
        let file = f["file"].as_str().expect("file");
        let path = corpus_dir.join(file);
        for (flag, key) in [(false, "sheets"), (true, "sheets_including_empty")] {
            let got = get_sheet_names(&path, flag);
            let want = names(&f[key]);
            if got != want {
                problems.push(format!("{file}: {key} expected {want:?} got {got:?}"));
            }
        }
    }
    for case in manifest["cases"].as_array().expect("cases") {
        let id = case["id"].as_str().expect("id");
        let dir = expected_dir.join(case["dir"].as_str().expect("dir"));
        let source = corpus_dir.join(case["source_file"].as_str().expect("source_file"));
        let sheet = case["sheet"].as_str();
        let info = json(&dir.join("case.json"));
        let is_csv = source
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("csv"));
        checked += 1;
        let result = read_grid(&source, sheet);
        let grid_path = dir.join("grid.json");
        if !grid_path.exists() {
            match result {
                Ok(_) => problems.push(format!("{id}: expected failure, got a grid")),
                Err(e) => {
                    let want = info["status"].as_str().unwrap_or("");
                    if e.status() != want {
                        problems.push(format!(
                            "{id}: status expected {want} got {} ({e})",
                            e.status()
                        ));
                    }
                }
            }
            continue;
        }
        if let Some(want) = info["grid_error"].as_str() {
            match result {
                Ok(_) => problems.push(format!(
                    "{id}: expected parser read failure ({want}), got a grid"
                )),
                Err(e) if e.status() != "parse_error" => {
                    problems.push(format!(
                        "{id}: expected parse_error got {} ({e})",
                        e.status()
                    ));
                }
                Err(_) => {}
            }
        } else {
            match result {
                Err(e) => problems.push(format!("{id}: read failed: {e}")),
                Ok(out) => {
                    diff_grids(&grid_of(&json(&grid_path)), &out.grid, &mut problems, id);
                    let want_name = if is_csv {
                        source.to_string_lossy().into_owned()
                    } else {
                        info["csv_file_name"]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| {
                                format!(
                                    "{}_{}.csv",
                                    source
                                        .file_stem()
                                        .map(|s| s.to_string_lossy())
                                        .unwrap_or_default(),
                                    sheet.unwrap_or("")
                                )
                            })
                    };
                    if out.csv_file_name != want_name {
                        problems.push(format!(
                            "{id}: csv_file_name expected {want_name:?} got {:?}",
                            out.csv_file_name
                        ));
                    }
                }
            }
        }
        if !is_csv {
            if let Some(sheet) = sheet {
                let want = fs::read(dir.join("intermediate.csv")).expect("intermediate.csv");
                match excel_sheet_csv_text(&source, sheet) {
                    Ok(got) if got.as_bytes() == want.as_slice() => {}
                    Ok(got) => {
                        let want = String::from_utf8_lossy(&want).into_owned();
                        let line = got
                            .lines()
                            .zip(want.lines())
                            .position(|(a, b)| a != b)
                            .unwrap_or(0);
                        let (a, b) = (
                            want.lines().nth(line).unwrap_or(""),
                            got.lines().nth(line).unwrap_or(""),
                        );
                        let at = a
                            .chars()
                            .zip(b.chars())
                            .position(|(x, y)| x != y)
                            .unwrap_or(0);
                        let ctx = |s: &str| {
                            s.chars()
                                .skip(at.saturating_sub(30))
                                .take(80)
                                .collect::<String>()
                        };
                        problems.push(format!(
                            "{id}: intermediate.csv differs at line {line} char {at}: expected {:?} got {:?}",
                            ctx(a),
                            ctx(b)
                        ));
                    }
                    Err(e) => problems.push(format!("{id}: csv text failed: {e}")),
                }
            }
        }
    }
    (checked, problems)
}

#[test]
fn checkpoint_a_fixture_corpus() {
    let root = repo_root();
    let (n, problems) = run_corpus(&root.join("fixtures/expected"), &root.join("fixtures"));
    assert!(n >= 13, "expected at least 13 cases, ran {n}");
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn checkpoint_a_synthetic_corpus() {
    let root = repo_root();
    let base = root.join("fixtures/synthetic-reader");
    let (n, problems) = run_corpus(&base.join("expected"), &base.join("corpus"));
    assert!(n > 50, "ran {n} cases");
    let shown: Vec<&String> = problems.iter().take(60).collect();
    assert!(
        problems.is_empty(),
        "{} problems (first 60):\n{}",
        problems.len(),
        shown
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
}
