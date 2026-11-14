"""Dump checkpoint A for the synthetic reader corpus with the legacy engine.

Usage: legacy/.venv/bin/python fixtures/synthetic-reader/oracle.py
Writes expected/manifest.json and expected/<id>/{grid.json,intermediate.csv,case.json}.
Every sheet (including ones that are not gradebooks) is a case.
"""
import contextlib
import io
import json
import os
import re
import shutil
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "legacy" / "src"))
import pandas as pd  # noqa: E402
from libs.excel_analyzer import GradebookProcessor  # noqa: E402

HERE = Path(__file__).resolve().parent
CORPUS = HERE / "corpus"
OUT = HERE / "expected"
UNSAFE = re.compile(r'[\x00-\x1f/\\:*?"<>|]')


def write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes((json.dumps(data, ensure_ascii=False, indent=1, allow_nan=False) + "\n").encode())


def read_grid(csv_file):
    with open(csv_file, "r", encoding="utf-8", newline="") as h:
        df = pd.read_csv(h, header=None, dtype=str, keep_default_na=False)
    return [[str(v) for v in row] for row in df.values.tolist()]


def run_case(source, sheet, case_dir):
    info = {"id": case_dir.name, "source_file": source.name, "sheet": sheet, "status": None, "ok": False,
            "grid_error": None, "csv_file_name": None}
    shutil.rmtree(case_dir, ignore_errors=True)
    case_dir.mkdir(parents=True)
    prev = os.getcwd()
    with tempfile.TemporaryDirectory(prefix="oracle_") as tmp:
        os.chdir(tmp)
        try:
            # The oracle works on a copy so that convert() never writes next to the corpus.
            local = Path(tmp) / source.name
            shutil.copyfile(source, local)
            with contextlib.redirect_stdout(io.StringIO()):
                p = GradebookProcessor(str(local), sheet_name=sheet)
                ok = p.convert()
            info["ok"] = bool(ok)
            info["status"] = (p.audit_report or {}).get("status")
            if ok:
                info["csv_file_name"] = Path(p.csv_file).name if sheet is None and source.suffix.lower() != ".csv" else (
                    p.csv_file if source.suffix.lower() != ".csv" else str(local))
                try:
                    write_json(case_dir / "grid.json", read_grid(p.csv_file))
                except Exception as exc:  # the parser's own read failed
                    info["grid_error"] = f"{type(exc).__name__}: {exc}"
                    info["status"] = "parse_error"
                if source.suffix.lower() != ".csv" or True:
                    shutil.copyfile(p.csv_file, case_dir / "intermediate.csv")
        finally:
            os.chdir(prev)
    write_json(case_dir / "case.json", info)
    return info


def main():
    shutil.rmtree(OUT, ignore_errors=True)
    OUT.mkdir()
    manifest = {"files": [], "cases": []}
    for path in sorted(CORPUS.iterdir()):
        if path.suffix.lower() == ".csv":
            targets = [(None, path.stem)]
        else:
            manifest["files"].append({
                "file": path.name,
                "sheets": GradebookProcessor.get_sheet_names(str(path)),
                "sheets_including_empty": GradebookProcessor.get_sheet_names(str(path), include_empty=True),
            })
            targets = [(s, f"{path.stem}__{s}") for s in GradebookProcessor.get_sheet_names(str(path), include_empty=True)]
        for sheet, raw in targets:
            cid = UNSAFE.sub("-", raw)
            info = run_case(path, sheet, OUT / cid)
            manifest["cases"].append({"id": cid, "dir": cid, "source_file": path.name, "sheet": sheet,
                                      "ok": info["ok"], "status": info["status"], "student_count": 0})
    write_json(OUT / "manifest.json", manifest)
    print(len(manifest["cases"]), "cases")


if __name__ == "__main__":
    main()
