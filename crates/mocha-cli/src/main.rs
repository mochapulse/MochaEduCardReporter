//! `mocha-cli`: run the Mocha engine without any UI.
//!
//! Subcommands: `sheets`, `inspect`, `generate`, `dump` (see `--help`).
//! Human output by default, `--json` for machine output. On error the exit
//! code is 1 and the error code (`FILE_NOT_FOUND`, ...) is printed.

mod dump;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use mocha_engine::{EngineError, ErrorCode, Gradebook};

const USAGE: &str = "\
mocha-cli: run the Mocha engine without a UI

USAGE:
  mocha-cli sheets <file> [--json]
  mocha-cli inspect <file> [--sheet NAME] [--json]
  mocha-cli generate <file> [--sheet NAME] --out DIR [--date ISO] [--students 1,2,5] [--json]
  mocha-cli dump <corpus-dir> --out DIR [--date ISO]

Exit code: 0 on success, 1 on engine error (code printed), 2 on usage error.";

/// A failure of the CLI: an engine error or a usage problem.
pub enum CliError {
    /// The engine failed.
    Engine(EngineError),
    /// Bad command line (exit code 2).
    Usage(String),
    /// Local I/O or serialization failure.
    Other(String),
}

impl From<EngineError> for CliError {
    fn from(e: EngineError) -> Self {
        CliError::Engine(e)
    }
}

/// Parsed command line options.
#[derive(Default)]
struct Opts {
    positional: Vec<String>,
    sheet: Option<String>,
    out: Option<PathBuf>,
    date: Option<String>,
    students: Option<Vec<i64>>,
    json: bool,
}

fn parse_opts(args: &[String]) -> Result<Opts, CliError> {
    let mut opts = Opts::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| CliError::Usage(format!("{name} needs a value")))
        };
        match arg.as_str() {
            "--sheet" => opts.sheet = Some(value("--sheet")?),
            "--out" => opts.out = Some(PathBuf::from(value("--out")?)),
            "--date" => opts.date = Some(value("--date")?),
            "--students" => {
                let raw = value("--students")?;
                let mut numbers = Vec::new();
                for part in raw.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                    numbers.push(
                        part.parse::<i64>().map_err(|_| {
                            CliError::Usage(format!("invalid student number: {part}"))
                        })?,
                    );
                }
                opts.students = Some(numbers);
            }
            "--json" => opts.json = true,
            flag if flag.starts_with("--") => {
                return Err(CliError::Usage(format!("unknown option: {flag}")));
            }
            _ => opts.positional.push(arg.clone()),
        }
    }
    Ok(opts)
}

fn one_path(opts: &Opts, what: &str) -> Result<PathBuf, CliError> {
    match opts.positional.as_slice() {
        [one] => Ok(PathBuf::from(one)),
        _ => Err(CliError::Usage(format!("expected exactly one {what}"))),
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, CliError> {
    serde_json::to_string_pretty(value).map_err(|e| CliError::Other(e.to_string()))
}

fn run(args: &[String]) -> Result<(), CliError> {
    let Some((command, rest)) = args.split_first() else {
        return Err(CliError::Usage("missing subcommand".into()));
    };
    let opts = parse_opts(rest)?;
    match command.as_str() {
        "sheets" => cmd_sheets(&one_path(&opts, "file")?, opts.json),
        "inspect" => cmd_inspect(&one_path(&opts, "file")?, opts.sheet.as_deref(), opts.json),
        "generate" => cmd_generate(&opts),
        "dump" => {
            let corpus = one_path(&opts, "corpus directory")?;
            let out = opts
                .out
                .clone()
                .ok_or_else(|| CliError::Usage("--out is required".into()))?;
            dump::run(&corpus, &out, opts.date.as_deref())
        }
        other => Err(CliError::Usage(format!("unknown subcommand: {other}"))),
    }
}

fn cmd_sheets(file: &Path, json: bool) -> Result<(), CliError> {
    let sheets = mocha_engine::list_sheets(file);
    if json {
        println!("{}", to_json(&sheets)?);
    } else if sheets.is_empty() {
        println!("(no sheets: CSV source or no usable sheet)");
    } else {
        for s in &sheets {
            match (&s.student_count, &s.error) {
                (Some(n), _) => println!("{}\t{} students", s.name, n),
                (None, Some(e)) => println!("{}\terror: {}", s.name, e),
                (None, None) => println!("{}", s.name),
            }
        }
    }
    Ok(())
}

fn cmd_inspect(file: &Path, sheet: Option<&str>, json: bool) -> Result<(), CliError> {
    let book = mocha_engine::inspect(file, sheet)?;
    if json {
        println!("{}", to_json(&book)?);
        return Ok(());
    }
    let m = &book.metadata;
    println!("File:        {}", m.file_name);
    println!("University:  {}", m.university_name);
    println!("Program:     {}", m.program_name);
    println!("Subject:     {}", m.materia_name);
    println!("Group:       {}", m.group_name);
    println!("Professor:   {}", m.professor_name);
    println!("Period:      {}", m.period_name);
    println!("First block: {} grades", m.first_twenty_grade_count);
    println!("Components:  {}", m.component_grade_numbers.len());
    println!("\nStudents ({}):", book.students.len());
    for s in &book.students {
        let number = s.student_number.map(|n| n.to_string()).unwrap_or_default();
        println!(
            "  {number:>3}  {:<12} {:<40} abse={} final={}",
            s.student_code, s.student_name, s.abse, s.definitive_grade
        );
    }
    if !book.warnings.is_empty() {
        println!("\nWarnings:");
        for w in &book.warnings {
            println!("  - {w}");
        }
    }
    Ok(())
}

fn cmd_generate(opts: &Opts) -> Result<(), CliError> {
    let file = one_path(opts, "file")?;
    let out = opts
        .out
        .clone()
        .ok_or_else(|| CliError::Usage("--out is required".into()))?;
    let book: Gradebook = mocha_engine::inspect(&file, opts.sheet.as_deref())?;
    let json = opts.json;
    let cancel = AtomicBool::new(false);
    let mut on_progress = |p: &mocha_engine::Progress| {
        if json {
            if let Ok(line) = serde_json::to_string(p) {
                println!("{line}");
            }
        } else {
            let number = p.student_number.map(|n| n.to_string()).unwrap_or_default();
            match (&p.pdf_path, &p.error) {
                (Some(path), _) => println!(
                    "[{}/{}] #{number} {} -> {}",
                    p.done,
                    p.total,
                    p.student_name,
                    path.display()
                ),
                (None, e) => println!(
                    "[{}/{}] #{number} {} FAILED: {}",
                    p.done,
                    p.total,
                    p.student_name,
                    e.clone().unwrap_or_default()
                ),
            }
        }
    };
    let outcome = mocha_engine::generate_reports(
        &book,
        opts.students.as_deref(),
        &out,
        opts.date.as_deref(),
        &mut on_progress,
        &cancel,
    )?;
    if json {
        println!("{}", to_json(&outcome)?);
    } else {
        println!(
            "Done: {} PDFs, {} failures",
            outcome.files.len(),
            outcome.failures.len()
        );
    }
    if outcome.failures.is_empty() {
        Ok(())
    } else {
        Err(CliError::Other(format!(
            "{} student(s) failed",
            outcome.failures.len()
        )))
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") || args.is_empty() {
        println!("{USAGE}");
        return ExitCode::from(if args.is_empty() { 2 } else { 0 });
    }
    let json = args.iter().any(|a| a == "--json");
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Engine(e)) => {
            report(json, e.code(), &e.to_string());
            ExitCode::from(1)
        }
        Err(CliError::Other(message)) => {
            report(json, ErrorCode::Internal, &message);
            ExitCode::from(1)
        }
        Err(CliError::Usage(message)) => {
            eprintln!("usage error: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn report(json: bool, code: ErrorCode, message: &str) {
    if json {
        let value = serde_json::json!({ "error": { "code": code.as_str(), "message": message } });
        println!("{value}");
    } else {
        eprintln!("error [{code}]: {message}");
    }
}
