#!/usr/bin/env python3
"""End-to-end parity check: Rust engine (mocha-cli dump) against the Python oracle tree.

Usage: legacy/.venv/bin/python tools/parity/check.py
           [--corpus fixtures] [--expected fixtures/expected] [--skip-pdf] [--keep DIR]

Builds mocha-cli (release), dumps the corpus into a temp dir (or --keep DIR) and compares:
manifest, grid.json (exact), parsed.json (exact, object key order matters),
rows.json (exact), case.json status fields and the PDFs (tools/pdfdiff/pdfdiff.py).
If the expected tree does not exist it is first produced with tools/oracle/dump.py.
Exit code 0 only if everything matches.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any, List, Optional

REPO = Path(__file__).resolve().parents[2]
PDFDIFF = REPO / "tools" / "pdfdiff" / "pdfdiff.py"
ORACLE = REPO / "tools" / "oracle" / "dump.py"
SPREADSHEETS = (".xls", ".xlsx", ".csv")


class Ordered(list):
    """A JSON object kept as an ordered list of pairs, so == is order-sensitive."""


def load(path: Path) -> Any:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle, object_pairs_hook=Ordered)


def first_diff(a: Any, b: Any, where: str = "$") -> Optional[str]:
    """Describe the first difference between two loaded JSON values, or None."""
    if isinstance(a, Ordered) and isinstance(b, Ordered):
        ka, kb = [k for k, _ in a], [k for k, _ in b]
        if ka != kb:
            return f"{where}: key order/set differs: {ka} vs {kb}"
        for (k, va), (_, vb) in zip(a, b):
            d = first_diff(va, vb, f"{where}.{k}")
            if d:
                return d
        return None
    if isinstance(a, list) and isinstance(b, list) and not isinstance(a, Ordered) and not isinstance(b, Ordered):
        if len(a) != len(b):
            return f"{where}: length {len(a)} vs {len(b)}"
        for i, (va, vb) in enumerate(zip(a, b)):
            d = first_diff(va, vb, f"{where}[{i}]")
            if d:
                return d
        return None
    if type(a) is not type(b) or a != b:
        return f"{where}: expected {a!r}, got {b!r}"
    return None


def to_plain(value: Any) -> Any:
    if isinstance(value, Ordered):
        return {k: to_plain(v) for k, v in value}
    if isinstance(value, list):
        return [to_plain(v) for v in value]
    return value


def cargo(*args: str) -> List[str]:
    return (["mise", "exec", "--", "cargo", *args] if shutil.which("mise") else ["cargo", *args])


def has_files(corpus: Path) -> bool:
    return corpus.is_dir() and any(p.is_file() and p.suffix.lower() in SPREADSHEETS for p in corpus.iterdir())


def compare_case(case: Any, exp_root: Path, act_root: Path, skip_pdf: bool) -> List[str]:
    """Returns the list of problems for one manifest case (empty = match)."""
    problems: List[str] = []
    cid = dict(case)["dir"]
    exp_dir, act_dir = exp_root / cid, act_root / cid
    if not act_dir.is_dir():
        return ["case directory missing in actual output"]
    for name in ("case.json", "grid.json", "parsed.json", "rows.json"):
        e, a = exp_dir / name, act_dir / name
        if not e.exists():
            if a.exists() and name != "case.json":
                problems.append(f"{name}: unexpected file produced")
            continue
        if not a.exists():
            problems.append(f"{name}: missing")
            continue
        if name == "case.json":
            keys = ("id", "source_file", "sheet", "status", "ok", "student_count")
            ed, ad = to_plain(load(e)), to_plain(load(a))
            diff = [k for k in keys if ed.get(k) != ad.get(k)]
            if diff:
                problems.append("case.json: " + ", ".join(f"{k} {ed.get(k)!r} vs {ad.get(k)!r}" for k in diff))
            continue
        d = first_diff(load(e), load(a))
        if d:
            problems.append(f"{name}: {d}")
    if not skip_pdf and (exp_dir / "pdf").is_dir():
        if not (act_dir / "pdf").is_dir():
            problems.append("pdf: directory missing")
        else:
            proc = subprocess.run(
                [sys.executable, str(PDFDIFF), str(exp_dir / "pdf"), str(act_dir / "pdf"), "--json"],
                capture_output=True, text=True,
            )
            try:
                report = json.loads(proc.stdout)
            except json.JSONDecodeError:
                problems.append(f"pdf: pdfdiff failed (exit {proc.returncode}): {proc.stderr.strip()[:300]}")
            else:
                for pair in report["pairs"]:
                    if not pair["ok"]:
                        problems.append(f"pdf {pair['name']}: " + "; ".join(pair["failures"][:3]))
    return problems


def check_corpus(corpus: Path, expected: Path, skip_pdf: bool, keep: Optional[Path], binary: Path) -> bool:
    if not has_files(corpus):
        print(f"[{corpus}] no gradebook files (.xls/.xlsx/.csv): nothing to compare")
        return True
    if not (expected / "manifest.json").exists():
        print(f"[{corpus}] expected tree missing, running the oracle into {expected} ...")
        subprocess.run([sys.executable, str(ORACLE), "--corpus", str(corpus), "--out", str(expected)], check=True)
    tmp_ctx = tempfile.TemporaryDirectory(prefix="parity-") if keep is None else None
    out = Path(tmp_ctx.name) if tmp_ctx else keep
    assert out is not None
    try:
        if keep is not None and out.exists():
            shutil.rmtree(out)
        out.mkdir(parents=True, exist_ok=True)
        cmd = [str(binary), "dump", str(corpus), "--out", str(out)]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        if proc.returncode != 0:
            print(f"[{corpus}] mocha-cli dump failed (exit {proc.returncode}): {proc.stderr.strip()[-500:]}")
            return False
        exp_manifest, act_manifest = load(expected / "manifest.json"), load(out / "manifest.json")
        em, am = to_plain(exp_manifest), to_plain(act_manifest)
        ok_all = True
        manifest_problems = []
        if em["timestamp_iso"] != am["timestamp_iso"]:
            manifest_problems.append("timestamp_iso differs")
        if em["files"] != am["files"]:
            manifest_problems.append("files/sheets differ")
        if em["cases"] != am["cases"]:
            ids_e, ids_a = [c["id"] for c in em["cases"]], [c["id"] for c in am["cases"]]
            if ids_e != ids_a:
                manifest_problems.append(f"case ids differ: only expected {sorted(set(ids_e) - set(ids_a))}, only actual {sorted(set(ids_a) - set(ids_e))}")
            else:
                for ce, ca in zip(em["cases"], am["cases"]):
                    if ce != ca:
                        manifest_problems.append(f"case {ce['id']}: {ce} vs {ca}")
        if manifest_problems:
            ok_all = False
            print(f"FAIL  manifest: {'; '.join(manifest_problems)}")
        else:
            print(f"OK    manifest ({len(em['cases'])} cases)")

        cases = dict(exp_manifest)["cases"]
        with ThreadPoolExecutor(max_workers=4) as pool:
            results = list(pool.map(lambda c: compare_case(c, expected, out, skip_pdf), cases))
        for case, problems in zip(cases, results):
            c = to_plain(case)
            summary = f"{c['id']} (ok={c['ok']} status={c['status']} students={c['student_count']})"
            if problems:
                ok_all = False
                print(f"FAIL  {summary}")
                for p in problems:
                    print(f"        - {p}")
            else:
                print(f"OK    {summary}")
        return ok_all
    finally:
        if tmp_ctx:
            tmp_ctx.cleanup()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--corpus", default="fixtures")
    ap.add_argument("--expected", default=None, help="default: <corpus>/expected")
    ap.add_argument("--skip-pdf", action="store_true")
    ap.add_argument("--keep", default=None, help="keep the Rust dump in this directory")
    args = ap.parse_args()
    corpus = Path(args.corpus).resolve()
    expected = Path(args.expected).resolve() if args.expected else corpus / "expected"
    keep = Path(args.keep).resolve() if args.keep else None

    if not has_files(corpus):
        print(f"[{corpus}] no gradebook files (.xls/.xlsx/.csv): nothing to compare")
        print("SUMMARY: 0 cases compared, parity OK (vacuous)")
        return 0

    print("Building mocha-cli (release) ...", flush=True)
    build = subprocess.run(cargo("build", "--release", "-p", "mocha-cli"), cwd=REPO)
    if build.returncode != 0:
        print("SUMMARY: build failed")
        return 1
    binary = REPO / "target" / "release" / "mocha-cli"
    ok = check_corpus(corpus, expected, args.skip_pdf, keep, binary)
    print("SUMMARY: " + ("all checks match" if ok else "MISMATCHES FOUND"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
