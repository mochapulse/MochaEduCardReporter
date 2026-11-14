#!/usr/bin/env python3
"""Decide whether two PDFs (or two directories of PDFs) are "the same".

Intended to compare reports produced by the legacy Python/ReportLab engine
with reports produced by the Rust engine. Checks, per pair of files:

1. Page count and per-page size.
2. Document Title and Author.
3. Font set (name, type, embedded flag).
4. Word text, word order and word positions (pdftotext -bbox).
5. Rendered pixels (pdftoppm + Pillow), as a percentage per page.

Usage:
    pdfdiff.py EXPECTED ACTUAL [--tol 0.5] [--dpi 100]
               [--max-pixel-diff 0.5] [--json] [--keep DIR]

EXPECTED and ACTUAL are either two PDF files or two directories. Directories
are compared by identical *.pdf file names; a file present on only one side
is a failure.

Exit codes: 0 all match, 1 at least one mismatch, 2 usage or tool error.
"""

import argparse
import difflib
import glob
import html.parser
import json
import os
import re
import subprocess
import sys
import tempfile

try:
    from PIL import Image, ImageChops
except ImportError:  # pragma: no cover - environment problem
    print("pdfdiff: error: Pillow is required (use legacy/.venv/bin/python)", file=sys.stderr)
    sys.exit(2)


PAGE_SIZE_TOL_PT = 0.01       # page size tolerance in points
PIXEL_CHANNEL_THRESHOLD = 48  # a pixel differs if any RGB channel differs by more
MAX_REPORTED_MISMATCHES = 10  # word mismatches listed per pair


class ToolError(Exception):
    """Usage problem or failing external tool; the program exits with code 2."""


class _BBoxParser(html.parser.HTMLParser):
    """Collects (text, (xMin, yMin, xMax, yMax)) per page from pdftotext -bbox.

    html.parser lowercases attribute names, so the keys are lowercase.
    """

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.pages = []
        self._box = None
        self._text = []

    def handle_starttag(self, tag, attrs):
        if tag == "page":
            self.pages.append([])
        elif tag == "word":
            attr = dict(attrs)
            self._box = tuple(float(attr[k]) for k in ("xmin", "ymin", "xmax", "ymax"))
            self._text = []

    def handle_data(self, data):
        if self._box is not None:
            self._text.append(data)

    def handle_endtag(self, tag):
        if tag == "word" and self._box is not None:
            if not self.pages:
                self.pages.append([])
            self.pages[-1].append(("".join(self._text).strip(), self._box))
            self._box = None


def run_tool(args):
    """Run an external tool and return its stdout; raise ToolError on failure."""
    try:
        proc = subprocess.run(args, capture_output=True, check=False)
    except FileNotFoundError:
        raise ToolError(f"required tool not found: {args[0]}")
    if proc.returncode != 0:
        stderr = proc.stderr.decode("utf-8", "replace").strip()
        raise ToolError(f"'{' '.join(args)}' failed (exit {proc.returncode}): {stderr}")
    return proc.stdout.decode("utf-8", "replace")


PAGE_SIZE_RE = re.compile(r"^Page\s+(\d+) size:\s+([\d.]+) x ([\d.]+) pts")


def pdf_info(path):
    """Return page count, per-page sizes (pt), Title and Author."""
    fields = {}
    for line in run_tool(["pdfinfo", path]).splitlines():
        if ":" in line:
            key, _, value = line.partition(":")
            fields.setdefault(key.strip(), value.strip())
    try:
        pages = int(fields["Pages"])
    except (KeyError, ValueError):
        raise ToolError(f"cannot read the page count of {path}")

    sizes = []
    if pages:
        out = run_tool(["pdfinfo", "-f", "1", "-l", str(pages), path])
        for line in out.splitlines():
            match = PAGE_SIZE_RE.match(line)
            if match:
                sizes.append((float(match.group(2)), float(match.group(3))))
    if len(sizes) != pages:
        raise ToolError(f"cannot read the page sizes of {path}")

    return {
        "pages": pages,
        "sizes": sizes,
        "title": fields.get("Title", ""),
        "author": fields.get("Author", ""),
    }


# Row layout of pdffonts: name, type, encoding, emb, sub, uni, object ID.
# The name never contains two consecutive spaces, so the columns can be
# split on runs of two or more spaces.
FONT_ROW_RE = re.compile(
    r"^(?P<name>.+?)\s{2,}(?P<type>.+?)\s{2,}(?P<enc>\S.*?)\s{2,}"
    r"(?P<emb>yes|no)\s+(?P<sub>yes|no)\s+(?P<uni>yes|no)\s+\d+\s+\d+\s*$"
)


def pdf_fonts(path):
    """Return the set of (font name, font type, embedded flag)."""
    lines = run_tool(["pdffonts", path]).splitlines()
    fonts = set()
    for line in lines[2:]:  # skip the header and the dashed separator
        if not line.strip():
            continue
        match = FONT_ROW_RE.match(line)
        if not match:
            raise ToolError(f"cannot parse pdffonts output line: {line!r}")
        fonts.add((match.group("name"), match.group("type"), match.group("emb") == "yes"))
    return fonts


def pdf_words(path):
    """Return a list per page of (text, (xMin, yMin, xMax, yMax)) tuples."""
    parser = _BBoxParser()
    parser.feed(run_tool(["pdftotext", "-bbox", path, "-"]))
    parser.close()
    return parser.pages


def format_box(box):
    return "(" + ", ".join(f"{v:.3f}" for v in box) + ")"


def compare_words(page_no, expected, actual, tol):
    """Compare the word sequences of one page.

    Words are aligned with difflib so that one inserted or missing word does
    not cascade into a mismatch for every later word. Text differences are
    reported as mismatches; positions are checked only for aligned words that
    have identical text.

    Returns (mismatches, max_delta) where max_delta covers all aligned pairs.
    """
    expected_texts = [text for text, _ in expected]
    actual_texts = [text for text, _ in actual]
    matcher = difflib.SequenceMatcher(None, expected_texts, actual_texts, autojunk=False)

    mismatches = []
    max_delta = 0.0
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "equal":
            for offset in range(i2 - i1):
                e_text, e_box = expected[i1 + offset]
                a_box = actual[j1 + offset][1]
                delta = max(abs(e - a) for e, a in zip(e_box, a_box))
                max_delta = max(max_delta, delta)
                if delta > tol:
                    mismatches.append({
                        "kind": "position",
                        "page": page_no,
                        "word": i1 + offset + 1,
                        "expected": {"text": e_text, "box": list(e_box)},
                        "actual": {"text": e_text, "box": list(a_box)},
                        "delta": delta,
                    })
        else:
            for offset in range(max(i2 - i1, j2 - j1)):
                e_idx = i1 + offset
                a_idx = j1 + offset
                e_item = expected[e_idx] if e_idx < i2 else None
                a_item = actual[a_idx] if a_idx < j2 else None
                mismatches.append({
                    "kind": "text",
                    "page": page_no,
                    "word": e_idx + 1,
                    "expected": None if e_item is None else {"text": e_item[0], "box": list(e_item[1])},
                    "actual": None if a_item is None else {"text": a_item[0], "box": list(a_item[1])},
                    "delta": None,
                })
    return mismatches, max_delta


def format_mismatch(m):
    def side(item):
        if item is None:
            return "nothing"
        return f"'{item['text']}' at {format_box(item['box'])}"

    if m["kind"] == "position":
        return (
            f"text_positions: page {m['page']} word {m['word']} '{m['expected']['text']}': "
            f"expected box {format_box(m['expected']['box'])}, "
            f"actual box {format_box(m['actual']['box'])}, delta {m['delta']:.4f} pt"
        )
    return (
        f"text_positions: page {m['page']} word {m['word']}: "
        f"expected {side(m['expected'])}, actual {side(m['actual'])}"
    )


def render_pages(pdf_path, out_dir, tag, dpi):
    """Render every page with pdftoppm. Returns {page number: PNG path}."""
    prefix = os.path.join(out_dir, tag)
    run_tool(["pdftoppm", "-r", str(dpi), "-png", pdf_path, prefix])
    pages = {}
    for path in glob.glob(glob.escape(prefix) + "-*.png"):
        match = re.search(r"-(\d+)\.png$", path)
        if match:
            pages[int(match.group(1))] = path
    return pages


def compare_raster(expected_pdf, actual_pdf, page_count, args, tmp_dir, stem):
    """Compare rendered pages. Returns (percent per page, failure messages)."""
    expected_pngs = render_pages(expected_pdf, tmp_dir, "expected", args.dpi)
    actual_pngs = render_pages(actual_pdf, tmp_dir, "actual", args.dpi)

    percents = []
    failures = []
    for page_no in range(1, page_count + 1):
        if page_no not in expected_pngs or page_no not in actual_pngs:
            raise ToolError(f"pdftoppm produced no image for page {page_no}")
        expected_img = Image.open(expected_pngs[page_no]).convert("RGB")
        actual_img = Image.open(actual_pngs[page_no]).convert("RGB")

        mask = None
        if expected_img.size != actual_img.size:
            percent = 100.0
            failures.append(
                f"raster: page {page_no} size differs in pixels: expected "
                f"{expected_img.size[0]}x{expected_img.size[1]}, "
                f"actual {actual_img.size[0]}x{actual_img.size[1]}"
            )
        else:
            diff = ImageChops.difference(expected_img, actual_img)
            red, green, blue = diff.split()
            max_channel = ImageChops.lighter(ImageChops.lighter(red, green), blue)
            mask = max_channel.point(lambda v: 255 if v > PIXEL_CHANNEL_THRESHOLD else 0)
            area = expected_img.size[0] * expected_img.size[1]
            percent = 100.0 * mask.histogram()[255] / area if area else 0.0

        percents.append(percent)
        if percent > args.max_pixel_diff:
            failures.append(
                f"raster: page {page_no} has {percent:.4f}% different pixels "
                f"(limit {args.max_pixel_diff}%)"
            )
            if args.keep:
                save_failed_page(args.keep, stem, page_no, expected_img, actual_img, mask)

    return percents, failures


def save_failed_page(keep_dir, stem, page_no, expected_img, actual_img, mask):
    """Save both renders and a diff image (differences painted red)."""
    os.makedirs(keep_dir, exist_ok=True)
    base = os.path.join(keep_dir, f"{stem}_p{page_no:03d}")
    expected_img.save(base + "_expected.png")
    actual_img.save(base + "_actual.png")
    if mask is not None:
        red = Image.new("RGB", expected_img.size, (255, 0, 0))
        Image.composite(red, expected_img, mask).save(base + "_diff.png")


def compare_pair(name, expected_pdf, actual_pdf, args, tmp_dir):
    """Run every check on one pair and collect all failures."""
    failures = []
    checks = {}
    stem = os.path.splitext(name)[0]

    exp_info = pdf_info(expected_pdf)
    act_info = pdf_info(actual_pdf)

    # 1. Pages and page sizes.
    pages_ok = exp_info["pages"] == act_info["pages"]
    checks["pages"] = pages_ok
    if not pages_ok:
        failures.append(f"pages: expected {exp_info['pages']}, actual {act_info['pages']}")

    sizes_ok = True
    for page_no, (exp_size, act_size) in enumerate(zip(exp_info["sizes"], act_info["sizes"]), 1):
        if any(abs(e - a) > PAGE_SIZE_TOL_PT for e, a in zip(exp_size, act_size)):
            sizes_ok = False
            failures.append(
                f"page_size: page {page_no} expected {exp_size[0]:.3f} x {exp_size[1]:.3f} pt, "
                f"actual {act_size[0]:.3f} x {act_size[1]:.3f} pt"
            )
    checks["page_size"] = sizes_ok

    # 2. Document info.
    info_ok = True
    for key in ("title", "author"):
        if exp_info[key] != act_info[key]:
            info_ok = False
            failures.append(
                f"document_info: {key} expected {exp_info[key]!r}, actual {act_info[key]!r}"
            )
    checks["document_info"] = info_ok

    # 3. Fonts.
    exp_fonts = pdf_fonts(expected_pdf)
    act_fonts = pdf_fonts(actual_pdf)

    def describe_font(font):
        name_, font_type, embedded = font
        return f"{name_} ({font_type}, {'embedded' if embedded else 'not embedded'})"

    missing = sorted(exp_fonts - act_fonts)
    unexpected = sorted(act_fonts - exp_fonts)
    checks["fonts"] = not missing and not unexpected
    failures.extend(f"fonts: missing in actual: {describe_font(f)}" for f in missing)
    failures.extend(f"fonts: unexpected in actual: {describe_font(f)}" for f in unexpected)

    # 4. Text and positions.
    exp_words = pdf_words(expected_pdf)
    act_words = pdf_words(actual_pdf)
    mismatches = []
    max_delta = 0.0
    for page_index in range(min(len(exp_words), len(act_words))):
        page_mismatches, page_delta = compare_words(
            page_index + 1, exp_words[page_index], act_words[page_index], args.tol
        )
        mismatches.extend(page_mismatches)
        max_delta = max(max_delta, page_delta)

    text_ok = not mismatches
    checks["text_positions"] = text_ok
    if not text_ok:
        for m in mismatches[:MAX_REPORTED_MISMATCHES]:
            failures.append(format_mismatch(m))
        if len(mismatches) > MAX_REPORTED_MISMATCHES:
            failures.append(
                f"text_positions: ... and {len(mismatches) - MAX_REPORTED_MISMATCHES} more mismatches"
            )
        failures.append(
            f"text_positions: {len(mismatches)} mismatches in total, "
            f"max position deviation {max_delta:.4f} pt (tolerance {args.tol} pt)"
        )

    # 5. Raster.
    percents, raster_failures = compare_raster(
        expected_pdf, actual_pdf, min(exp_info["pages"], act_info["pages"]), args, tmp_dir, stem
    )
    checks["raster"] = not raster_failures
    failures.extend(raster_failures)

    return {
        "name": name,
        "ok": all(checks.values()),
        "checks": checks,
        "failures": failures,
        "max_position_delta": max_delta,
        "pixel_diff_percent": percents,
    }


def missing_result(name, side):
    """Result for a file present on only one side of a directory comparison."""
    where = "missing in actual directory" if side == "expected" else "missing in expected directory"
    return {
        "name": name,
        "ok": False,
        "checks": {},
        "failures": [f"file {where}"],
        "max_position_delta": None,
        "pixel_diff_percent": [],
    }


def discover_pairs(expected, actual):
    """Return a list of (name, expected path or None, actual path or None)."""
    for label, path in (("EXPECTED", expected), ("ACTUAL", actual)):
        if not os.path.exists(path):
            raise ToolError(f"{label} does not exist: {path}")

    if os.path.isdir(expected) and os.path.isdir(actual):
        def pdf_names(directory):
            return {
                entry for entry in os.listdir(directory)
                if entry.lower().endswith(".pdf") and os.path.isfile(os.path.join(directory, entry))
            }

        exp_names = pdf_names(expected)
        act_names = pdf_names(actual)
        if not exp_names and not act_names:
            raise ToolError("no *.pdf files found in either directory")
        pairs = []
        for name in sorted(exp_names | act_names):
            exp_path = os.path.join(expected, name) if name in exp_names else None
            act_path = os.path.join(actual, name) if name in act_names else None
            pairs.append((name, exp_path, act_path))
        return pairs

    if os.path.isfile(expected) and os.path.isfile(actual):
        exp_name = os.path.basename(expected)
        act_name = os.path.basename(actual)
        name = exp_name if exp_name == act_name else f"{exp_name} vs {act_name}"
        return [(name, expected, actual)]

    raise ToolError("EXPECTED and ACTUAL must both be files or both be directories")


def print_human(results, compared, failed):
    for result in results:
        name = result["name"]
        if result["ok"]:
            notes = []
            if result["max_position_delta"] is not None and result["checks"].get("text_positions"):
                notes.append(f"max position delta {result['max_position_delta']:.4f} pt")
            if result["pixel_diff_percent"]:
                notes.append(f"max pixel diff {max(result['pixel_diff_percent']):.4f}%")
            suffix = f" ({'; '.join(notes)})" if notes else ""
            print(f"OK    {name}{suffix}")
            continue

        print(f"FAIL  {name}")
        for failure in result["failures"]:
            print(f"        - {failure}")
        if result["pixel_diff_percent"]:
            per_page = ", ".join(f"p{i}={p:.4f}%" for i, p in enumerate(result["pixel_diff_percent"], 1))
            print(f"        pixel diff per page: {per_page}")
    print(f"{compared} compared, {failed} failed")


def build_parser():
    parser = argparse.ArgumentParser(
        description="Compare two PDFs (or two directories of PDFs) for text, fonts, "
                    "positions, page geometry and rendered pixels.",
    )
    parser.add_argument("expected", help="expected PDF file or directory")
    parser.add_argument("actual", help="actual PDF file or directory")
    parser.add_argument("--tol", type=float, default=0.5,
                        help="maximum word coordinate difference in points (default 0.5)")
    parser.add_argument("--dpi", type=int, default=100,
                        help="render resolution for the raster check (default 100)")
    parser.add_argument("--max-pixel-diff", type=float, default=0.5,
                        help="maximum percent of different pixels per page (default 0.5)")
    parser.add_argument("--json", action="store_true", help="print a JSON report")
    parser.add_argument("--keep", metavar="DIR",
                        help="save renders and a diff image for each failing raster page")
    return parser


def main(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.tol < 0:
        parser.error("--tol must be >= 0")
    if args.dpi <= 0:
        parser.error("--dpi must be > 0")
    if args.max_pixel_diff < 0:
        parser.error("--max-pixel-diff must be >= 0")

    try:
        results = []
        for name, exp_path, act_path in discover_pairs(args.expected, args.actual):
            if exp_path is None:
                results.append(missing_result(name, "expected"))
            elif act_path is None:
                results.append(missing_result(name, "actual"))
            else:
                with tempfile.TemporaryDirectory(prefix="pdfdiff-") as tmp_dir:
                    results.append(compare_pair(name, exp_path, act_path, args, tmp_dir))
    except ToolError as exc:
        print(f"pdfdiff: error: {exc}", file=sys.stderr)
        return 2

    failed = sum(1 for result in results if not result["ok"])
    if args.json:
        report = {"pairs": results, "compared": len(results), "failed": failed}
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print_human(results, len(results), failed)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
