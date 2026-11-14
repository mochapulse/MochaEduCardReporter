# PARITY_NOTES

Behaviors of the legacy Python engine (`legacy/src/libs/excel_analyzer.py`)
that the Rust engine reproduces on purpose. See `PLAN_REDO_CODE.md`, section 2.
Nothing here is "fixed"; changing any item is a separate decision.

Oracle engine SHA-256: `4d4c1e7ae03e323c0f24d171ceb53455476d3f6254927846072773eff157464f`

## Reading (checkpoint A)

- The first sheet row is treated as a header row by pandas. Empty header cells
  become `Unnamed: N` (N = 0-based column index). So grid row 0 can read
  `UNIVERSITÉ DE CALDAS, Unnamed: 1, ...`.
- A column of whole numbers without blanks prints as `4`; with blanks it becomes
  float and prints as `4.0`.
- `float_format="%.1f"` is not applied to mixed (text + number) columns: a value
  such as `3.8428571428571425` keeps full precision in the intermediate CSV and
  is only rounded later by `to_grade_str`.
- An empty CSV fails conversion with status `corrupted`.
- Duplicate header names are renamed pandas-style (`a,a,a.1` → `a,a.2,a.1`).
- Text cells that look like missing values (`NA`, `N/A`, `NULL`, `nan`, `None`,
  `#N/A`, ...) are treated as blank during column type inference.
- Text that looks like a whole number is converted (`"007"` prints `7`).
- Object (mixed) columns print each value's own text form; whole-number,
  float, bool and datetime columns each have their own print rule
  (see `reader/normalize.rs`).
- CSV reading follows pandas' tokenizer, not a generic CSV library: blank and
  whitespace-only lines are skipped, a later row longer than the first is an
  error, shorter rows are padded with `""`.
- Verified on 13 fixture cases plus 347 synthetic cases; the intermediate CSV is
  checked byte for byte for Excel sources.

### Not verified (no evidence available)

- `.xls` edge cases beyond the two real fixtures (no `.xls` writer available);
  date/time cells in `.xls` are assumed to behave like `.xlsx`.
- `.xlsx` date/time detection relies on calamine instead of openpyxl.
- A CSV whose blank lines end with a lone `\r` (old Mac line endings).
- Integers above 64 bits mixed with booleans in one column.

## Parsing (checkpoint B)

- `grade_labels` are collected from every cell of the sheet; a later match for
  the same number overwrites the earlier one. Key order is first-seen order, not
  numeric order.
- `component_grades` carries a synthetic key `"1-N average"`; weighted values
  use keys like `"1-N weighted 20%"` and `"9 weighted 25%"`.
- `first_block_weight_label` can be `20%` while components use other weights.
- Absences go through `to_grade_str`, so `4` becomes `"4.0"`.
- Carnet layout: university, program, professor and student code are empty;
  the group is the sheet label taken from the intermediate CSV name; the period
  is the first 4-digit number in that name; multi-line item labels are joined
  with spaces and keep trailing punctuation.
- Student numbers are replaced by row order when the sheet's numbering is wrong,
  with a warning.

## Python semantics (helpers)

- `\s`, `\d`, `\w` differ between Python `re` and Rust `regex`; patterns use
  explicit class tables dumped from Python 3.12 (Unicode 15.0).
- `float()` trims only ASCII whitespace, accepts `1_0`, `inf`, `nan`, and
  non-ASCII decimal digits; rejects `4,5`, `0x10`, `_1`.
- `f"{x:.1f}"` rounds the exact binary value (`3.25` → `3.2`, `3.35` → `3.4`).
- `str.lower()` vs Rust `to_lowercase` may differ only for rare characters
  across Unicode versions (not present in gradebooks).

## PDF (checkpoint D)

Phase 1 gate result: **GO** with `pdf-writer`. 232 of 232 fixture PDFs match
the oracle (max position delta 0.00005 pt, pixel difference 0 %), verified
independently with `tools/pdfdiff`.

ReportLab behaviors mirrored on purpose:

- Body cells are Paragraphs, so table `ALIGN` does not center their text; the
  header row is plain strings and is centered.
- The `NOTE DÉFINITIVE` label sits under a span and is never drawn; only the
  final grade shows in that row. This is how the current PDFs look.
- Spanned cells are excluded from row-height calculation and drawn centered in
  the whole span; grid lines are interrupted across spans.
- Rows inside the first block cannot be split across pages (because of the `%`
  and `MOY` spans). A first block taller than one page makes ReportLab raise a
  layout error; Rust returns `PdfError::Layout` for the same input.
- A table that does not fit under the header text moves whole to the next page.
- Paragraph first baseline is `height - fontSize`; line fitting allows 5 % space
  shrinkage and applies negative word spacing when used.
- Row positions use ReportLab's compensated summation; spacing between
  flowables is `max(spaceAfter, spaceBefore)`.
- Characters outside WinAnsi fall back per character to Symbol, then
  ZapfDingbats.
- Grid lines use round caps and joins.

Accepted differences (not visible):

- The file is uncompressed PDF 1.7, has no CreationDate, and Producer is
  `MochaEduCardReporter` instead of ReportLab.

Not implemented (cannot occur with current gradebooks, returns an error or
falls back):

- Inline formatting tags in labels (`<b>`, `<i>`, `<font>`) return
  `PdfError::Markup`; ReportLab would render them.
- Soft hyphen (U+00AD) handling.
- Date line: only `YYYY-MM-DD` with optional time and offset is parsed; other
  ISO forms fall back to the raw string.

## Known differences from Python on malformed input

Where Python raises an exception, Rust returns a typed error instead:

| Python                                              | Rust                          |
| --------------------------------------------------- | ----------------------------- |
| `ValueError` "Could not find a supported ... layout" | `ParseError::NoSupportedLayout` |
| `IndexError` (carnet header under 5 columns)        | `ParseError::IndexOutOfRange` |
| `ValueError` on integers over 4300 digits           | `ParseError::IntegerTooLarge` |
| `OverflowError` on `int(float("inf"))`              | `ParseError::InvalidGradeNumber` |
| pandas read failures                                | `ReadError` with the same status text (`corrupted`, `empty`, ...) |

Additional parser quirks preserved: label keys are kept as written (`"05"`),
a repeated grade number overwrites in place, the carnet block ends only at a
blank row or `note du cours`, and the weights row equals the header row when
the header is the first row.
