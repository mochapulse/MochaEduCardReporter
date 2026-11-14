"""Run the legacy Python engine (the parity oracle) over a corpus and dump checkpoints.

Per case: grid.json (A), parsed.json (B), rows.json (C), pdf/ (D), case.json.
Top level: manifest.json. Never edits legacy/. JSON output is deterministic.

Usage: legacy/.venv/bin/python tools/oracle/dump.py [--corpus fixtures] [--out fixtures/expected]
"""
from __future__ import annotations

import argparse
import contextlib
import hashlib
import io
import json
import logging
import os
import re
import shutil
import sys
import tempfile
from dataclasses import asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

REPO = Path(__file__).resolve().parents[2]
ENGINE = REPO / "legacy" / "src" / "libs" / "excel_analyzer.py"
sys.path.insert(0, str(REPO / "legacy" / "src"))

import pandas as pd  # noqa: E402
from libs.excel_analyzer import (  # noqa: E402
    GradebookParser,
    GradebookProcessor,
    PDFGenerator,
)

TIMESTAMP_ISO = "2026-01-15 10:00:00"
UNSAFE = re.compile(r'[\x00-\x1f/\\:*?"<>|]')


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(data, ensure_ascii=False, indent=1, allow_nan=False, default=_json_default)
    path.write_bytes((text + "\n").encode("utf-8"))


def _json_default(obj: Any) -> Any:
    if hasattr(obj, "item"):  # numpy scalars
        return obj.item()
    if isinstance(obj, Path):
        return str(obj)
    raise TypeError(f"not JSON-safe: {type(obj)}")


def safe_id(text: str) -> str:
    return UNSAFE.sub("-", text)


class Capture(logging.Handler):
    def __init__(self) -> None:
        super().__init__(logging.WARNING)
        self.messages: List[str] = []

    def emit(self, record: logging.LogRecord) -> None:
        self.messages.append(record.getMessage())


def read_grid(csv_file: str) -> List[List[str]]:
    with open(csv_file, "r", encoding="utf-8", newline="") as handle:
        df = pd.read_csv(handle, header=None, dtype=str, keep_default_na=False)
    return [[str(v) for v in row] for row in df.values.tolist()]


def normalize_audit(report: Optional[Dict[str, Any]], source: Path) -> Optional[Dict[str, Any]]:
    """Make the audit report portable: absolute CSV source paths become the bare file name."""
    if report is None:
        return None
    out = json.loads(json.dumps(report, default=_json_default))
    path = out.get("conversion_path")
    if isinstance(path, str) and os.path.isabs(path):
        out["conversion_path"] = Path(path).name
    return out


def run_case(source: Path, sheet: Optional[str], case_dir: Path) -> Dict[str, Any]:
    case_id = case_dir.name
    info: Dict[str, Any] = {
        "id": case_id, "source_file": source.name, "sheet": sheet,
        "status": None, "ok": False, "student_count": 0, "audit_report": None,
    }
    if case_dir.exists():
        shutil.rmtree(case_dir)
    case_dir.mkdir(parents=True)

    prev_cwd = os.getcwd()
    with tempfile.TemporaryDirectory(prefix="oracle_") as tmp:
        os.chdir(tmp)
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                processor = GradebookProcessor(str(source), sheet_name=sheet)
                ok = processor.convert()
            info["ok"] = bool(ok)
            info["audit_report"] = normalize_audit(processor.audit_report, source)
            info["status"] = (processor.audit_report or {}).get("status")
            if not ok:
                write_json(case_dir / "case.json", info)
                return info

            csv_file = processor.csv_file
            write_json(case_dir / "grid.json", read_grid(csv_file))
            shutil.copyfile(csv_file, case_dir / "intermediate.csv")

            logger = logging.getLogger(f"oracle.{case_id}")
            logger.setLevel(logging.DEBUG)
            logger.propagate = False
            capture = Capture()
            logger.addHandler(capture)
            metadata, students, error = None, [], None
            try:
                with contextlib.redirect_stdout(io.StringIO()):
                    metadata, students, _ = GradebookParser(logger=logger).parse(csv_file)
            except Exception as exc:  # parse raises on failure
                error = f"{type(exc).__name__}: {exc}"
            logger.removeHandler(capture)
            write_json(case_dir / "parsed.json", {
                "metadata": asdict(metadata) if metadata else None,
                "students": [asdict(s) for s in students],
                "warnings": capture.messages,
                "error": error,
            })
            info["student_count"] = len(students)
            if error is not None:
                info["status"] = "parse_error"
                write_json(case_dir / "case.json", info)
                return info

            with contextlib.redirect_stdout(io.StringIO()):
                rows_gen = PDFGenerator(metadata, students, str(Path(tmp) / "rows_out"))
                rows_out = []
                for s in students:
                    rows, meta = rows_gen._build_student_rows(s)
                    rows_out.append({
                        "student_number": s.student_number,
                        "student_name": s.student_name,
                        "rows": rows,
                        "meta": meta,
                    })
            write_json(case_dir / "rows.json", rows_out)

            pdf_dir = case_dir / "pdf"
            if pdf_dir.exists():
                shutil.rmtree(pdf_dir)
            with contextlib.redirect_stdout(io.StringIO()):
                PDFGenerator(metadata, students, str(pdf_dir), timestamp_iso=TIMESTAMP_ISO).generate()
        finally:
            os.chdir(prev_cwd)
    write_json(case_dir / "case.json", info)
    return info


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", default="fixtures")
    ap.add_argument("--out", default="fixtures/expected")
    args = ap.parse_args()
    corpus = Path(args.corpus).resolve()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)

    files = sorted(p for p in corpus.iterdir()
                   if p.is_file() and p.suffix.lower() in (".xls", ".xlsx", ".csv"))
    manifest: Dict[str, Any] = {
        "timestamp_iso": TIMESTAMP_ISO,
        "engine_sha256": hashlib.sha256(ENGINE.read_bytes()).hexdigest(),
        "files": [],
        "cases": [],
    }
    seen_ids = set()
    case_dirs = []
    for path in files:
        if path.suffix.lower() == ".csv":
            targets = [(None, path.stem)]
        else:
            sheets = GradebookProcessor.get_sheet_names(str(path))
            manifest["files"].append({
                "file": path.name,
                "sheets": sheets,
                "sheets_including_empty": GradebookProcessor.get_sheet_names(str(path), include_empty=True),
            })
            targets = [(s, f"{path.stem}__{s}") for s in sheets]
        for sheet, raw_id in targets:
            case_id = safe_id(raw_id)
            if case_id in seen_ids:
                raise SystemExit(f"case id collision: {case_id}")
            seen_ids.add(case_id)
            info = run_case(path, sheet, out / case_id)
            case_dirs.append(case_id)
            manifest["cases"].append({
                "id": case_id, "dir": case_id, "source_file": path.name, "sheet": sheet,
                "ok": info["ok"], "status": info["status"], "student_count": info["student_count"],
            })
            print(f"{case_id}: ok={info['ok']} status={info['status']} students={info['student_count']}",
                  file=sys.stderr)

    # remove stale case directories from earlier runs
    for child in out.iterdir():
        if child.is_dir() and child.name not in seen_ids:
            shutil.rmtree(child)
    write_json(out / "manifest.json", manifest)
    return 0


if __name__ == "__main__":
    sys.exit(main())
