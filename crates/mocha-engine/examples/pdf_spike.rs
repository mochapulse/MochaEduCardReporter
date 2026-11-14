//! Phase 1 spike: render every student of a `parsed.json` with the Rust PDF engine.
//!
//! Usage: `cargo run -p mocha-engine --example pdf_spike -- <parsed.json> <out_dir> [timestamp]`
//! (default timestamp `2026-01-15 10:00:00`, the oracle's).

use std::path::Path;

use mocha_engine::model::{FileMetadata, StudentGrades};
use mocha_engine::pdf::generate_student_pdf;

#[derive(serde::Deserialize)]
struct Parsed {
    metadata: FileMetadata,
    students: Vec<StudentGrades>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: pdf_spike <parsed.json> <out_dir> [timestamp]");
        std::process::exit(2);
    }
    let ts = args.get(3).map_or("2026-01-15 10:00:00", String::as_str);
    let parsed: Parsed = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let out = Path::new(&args[2]);
    for s in &parsed.students {
        generate_student_pdf(&parsed.metadata, s, out, Some(ts))?;
    }
    println!(
        "{} PDFs written to {}",
        parsed.students.len(),
        out.display()
    );
    Ok(())
}
