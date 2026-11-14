# PLAN_REDO_CODE — MochaEduCardReporter 1.0 (native Tauri rewrite)

Status: **approved; phases 0–10 executed, phase 11 partially executed — only human steps remain.**
Date: 2026-11-14 (revision 5: real run, Linux bundles, CI/release workflows, updater wired, release-build parity clean)

---

## 1. Goal

Rebuild the application as a single, self-contained native desktop app
(Tauri v2 + Rust) for Windows 11 and Linux. The engine is **re-implemented in
Rust with identical behavior**: same reading rules, same regular expressions,
same lookups, same tables, same PDF layout. The shipped app contains no Python.

| Is a problem (fix it)                          | Is NOT a problem (leave it)                     |
| ---------------------------------------------- | ----------------------------------------------- |
| Code quality                                   | CPU / RAM / disk usage, installer size          |
| Ugly, bare interface                           | What the PDFs look like and contain             |
| Missing features and quality-of-life           | How gradebooks are interpreted                  |
| Installation friction                          |                                                 |

### Target user

A 50-year-old French teacher with basic PC skills. Uses the app a few times per
term, not daily, so nothing can rely on remembering "how it worked last time".
Spanish-speaking; the report cards themselves are in French.

- The interface language is **Spanish**; the PDF content stays in French.
- One obvious next step per screen. No jargon, no hidden gestures, no settings required to get work done.
- Nothing destructive happens without a plain-language confirmation.

---

## 2. Hard constraint: behavior parity

The Python engine (`src/libs/excel_analyzer.py`) is the **specification**. The
Rust engine must produce the same result for the same input. "Same" is defined
and tested at four checkpoints:

| # | Checkpoint                        | Must match                                                                  |
| - | --------------------------------- | --------------------------------------------------------------------------- |
| A | Normalized cell grid              | Exactly, cell by cell (what the Python parser sees after its CSV round trip) |
| B | Parsed metadata and student data  | Exactly (JSON equality)                                                      |
| C | Table rows built for each student | Exactly (text, order, weights, spans)                                        |
| D | The PDF                           | Same text, same fonts, same positions within 0.5 pt, same colors and lines   |

Byte-identical PDF files are not a goal (ReportLab embeds its own IDs and
timestamps); visual and textual identity is.

### 2.1 Porting rules

1. **Line-by-line port, not a redesign.** One Rust function per Python function, same names, same order of operations. Each carries a comment pointing to the Python lines it mirrors.
2. **Regex patterns copied verbatim** into one file (`patterns.rs`), each tested against the Python `re` result over a text corpus.
3. **Python semantics are emulated explicitly** in a small helper module, never approximated ad hoc (see 3.3).
4. **Quirks are preserved, not fixed.** If the Python engine does something odd (for example, taking labels from anywhere in the sheet with "last one wins"), the Rust engine does the same. Any quirk found is written down in `PARITY_NOTES.md`; changing one is a separate decision for after 1.0.
5. **The Python engine stays in the repository as a development-only oracle** (`tools/oracle/`), never shipped, used to produce the expected results for the four checkpoints. It is not edited.

### 2.2 Corpus

- The fixtures already in `templates/`.
- **Real gradebooks supplied by the owner**, placed in a git-ignored folder (`corpus-private/`), so parity is proven on the "many real cases" the current engine was validated with. The wider this corpus, the stronger the guarantee.
- Synthetic edge cases: empty sheets, wrong numbering, missing columns, accents, merged cells, formulas, numbers stored as text.

---

## 3. Architecture

```
┌──────────────────── one native executable (Tauri) ────────────────────┐
│  Frontend (WebView)                 Rust                               │
│  React + TypeScript   ──typed IPC──► app shell ──► mocha-engine crate  │
│  screens, PDF viewer                commands,      read → parse → PDF  │
│                                     settings       (no I/O to the UI)  │
└───────────────────────────────────────────────────────────────────────┘
```

No sidecar, no second process, no runtime to install.

### 3.1 Workspace layout

The old Python app moves, unedited, to `legacy/` with a single `git mv` (file
history preserved); the new code owns the repository root, which is where Tauri
expects `src/` and `src-tauri/`. `legacy/` doubles as the parity oracle. The
state before the move is the existing tag `v0.2.0`.

```
legacy/                       old Python app, complete and unedited (the oracle)
  src/  test/  scripts/  app.spec  requirements.txt  pytest.ini
fixtures/                     former templates/ (used by both engines)
  expected/                   oracle output for checkpoints A, B, C and PDFs
corpus-private/               owner's real gradebooks (git-ignored)
crates/
  mocha-engine/               pure Rust library, no Tauri dependency
    src/
      model.rs                FileMetadata, StudentGrades (same fields)
      pycompat.rs             Python-semantics helpers
      patterns.rs             every regex, verbatim
      text.rs                 clean_text, to_grade_str, to_float, format_* ...
      reader/
        workbook.rs           .xls / .xlsx via calamine
        csv.rs                .csv
        normalize.rs          emulation of the pandas → CSV → pandas round trip
        sheets.rs             get_sheet_names / _sheet_has_gradebook_header
      parser/
        tabular.rs            GradebookParser.parse (2019 / 2025 layouts)
        carnet.rs             _parse_carnet_blocks and helpers
        labels.rs             _extract_grade_labels
      pdf/
        metrics.rs            Helvetica / Helvetica-Bold width tables (AFM)
        paragraph.rs          ReportLab Paragraph: wrapping, <br/>, leading
        table.rs              ReportLab Table: heights, spans, grid, page split
        document.rs           SimpleDocTemplate frame, story flow, metadata
        report.rs             _build_document_story, _build_student_rows, styles
        writer.rs             PDF objects via pdf-writer
      processor.rs            GradebookProcessor equivalent
      error.rs
    tests/                    parity tests against oracle output
  mocha-cli/                  dev tool: run the engine, dump checkpoints, diff
src-tauri/                    app shell: commands, settings, history, updater
src/                          frontend
tools/
  oracle/                     script that runs legacy/ and dumps the checkpoints
  pdfdiff/                    renders two PDFs and compares them
install/                      install.ps1, install.sh
.github/workflows/            ci.yml, release.yml
```

The old release workflow (which builds the Python app on every `v*` tag) is
disabled when the move happens, and replaced in Phase 10. At the end, `legacy/`
is either deleted or kept as a regression reference (dev-only, never shipped).

### 3.2 Reading spreadsheets

| Need                   | Crate      | Note                                                       |
| ---------------------- | ---------- | ---------------------------------------------------------- |
| `.xlsx` and `.xls`     | `calamine` | pure Rust, reads cached formula values like pandas does    |
| `.csv`                 | `csv`      | UTF-8, same quoting rules as the Python reader              |

**The delicate part is `normalize.rs`.** The Python engine never parses the
spreadsheet directly: it reads it with pandas, writes a CSV with one-decimal
float formatting, and reads that CSV back as text. The parser's input is the
result of that round trip, so the Rust reader must reproduce its effects:

- the first row becomes a header row; empty header cells become `Unnamed: N`, duplicates get `.1`, `.2` suffixes;
- a column holding only whole numbers and no blanks is written as `4`; with blanks it becomes `4.0`; real decimals are rounded to one decimal;
- columns mixing text and numbers keep each value's own text form;
- the grid starts at cell A1 even if the first rows or columns are empty;
- dates, booleans and error cells take pandas' text form;
- fully empty rows are kept as rows of empty cells.

This list is the starting hypothesis; the exact rules are confirmed in Phase 2
by comparing against the oracle's grid (checkpoint A) on the whole corpus.

### 3.3 Python-semantics helpers (`pycompat.rs`)

| Python behavior                         | Rust helper                                                    |
| --------------------------------------- | -------------------------------------------------------------- |
| `float(text)` accepted grammar          | `py_float` (same accepted and rejected strings)                |
| `f"{x:.1f}"`                            | `fmt_1f` (same rounding of the binary value)                   |
| `f"{x:g}"`                              | `fmt_g`                                                        |
| `str.strip()`, `re` `\s`, `\d`, `\w`    | same Unicode classes; differences listed and tested            |
| `re.match` / `re.fullmatch` / `search`  | explicit anchoring with the same pattern text                  |
| `dict` insertion order and overwrite    | `IndexMap`                                                     |
| `sorted(..., key=int)` stability        | stable sort                                                    |
| `pd.isna`, empty-string handling        | explicit `Cell` enum (`Empty`, `Text`, `Number`, ...)          |

### 3.4 PDF generation

Crate: **`pdf-writer`** (low-level, maintained, pure Rust) to emit the PDF, with
a small layout module that **re-implements the ReportLab behaviors this report
actually uses**. A higher-level typesetting crate would be less code but would
choose its own line breaks, fonts and row heights, so the result would look
"similar", not the same. Here the layout is small and fixed (a few paragraphs
and one table), so mirroring it exactly is feasible.

What is mirrored:

- **Fonts:** the standard PDF fonts Helvetica and Helvetica-Bold, not embedded, WinAnsi encoding — exactly what ReportLab emits. Character widths come from the standard Adobe metric tables for those two fonts.
- **Paragraphs:** greedy word wrapping by measured width, `<br/>` line breaks, per-style font size, leading, color, alignment and space-after (including inherited values from ReportLab's sample styles, e.g. the title's 22 pt leading).
- **Page frame:** A4, margins 10 / 10 / 8 / 8 mm, plus ReportLab's default 6 pt frame padding.
- **Table:** fixed column widths (10, 104, 18, 16, 18 mm), centered in the frame, row height from the tallest cell plus paddings, vertical centering, merged cells (`SPAN`) for the first block's `%` and `MOY` columns and for the final row, 0.5 pt grid, thicker separator lines, header background, header row repeated on page breaks.
- **Content:** same title, header lines, student block, row numbering, colors (`#7A2CB8`, `#3F5F23`, red final row), footer text, document title and author metadata.
- **File names:** `NN_Student_Name.pdf`, same sanitizing rule.

This is the highest-risk piece, so it is proven first, in a spike (Phase 1),
before anything else is built. If the spike cannot meet checkpoint D with
`pdf-writer`, the fallback is `printpdf` with the same layout module.

### 3.5 App shell (Rust, `src-tauri`)

Typed commands, engine work on a background thread so the window never freezes:

`list_sheets`, `inspect_gradebook`, `preview_student`, `generate_reports`
(progress streamed over a Tauri `Channel`, cancel through a shared flag),
`read_pdf`, `get_settings` / `save_settings`, `get_history`,
`reveal_in_folder`, `open_externally`, `check_for_update`.

The engine exposes per-student generation, so progress, cancel, choosing a
subset of students and single-report preview need no special tricks.

### 3.6 Packages chosen for quality and features

| Area                | Package                                   | Why                                                          |
| ------------------- | ----------------------------------------- | ------------------------------------------------------------ |
| Typed IPC           | `tauri-specta` + `specta`                 | TypeScript types generated from Rust commands; no hand-kept duplicates |
| Native dialogs      | `tauri-plugin-dialog`                     | OS file and folder pickers                                    |
| Open / reveal       | `tauri-plugin-opener`                     | "Abrir carpeta", "Abrir con otro programa"                    |
| Persistence         | `tauri-plugin-store`                      | settings, recent files, history                               |
| Logs                | `tauri-plugin-log`                        | rotating log files                                            |
| Window              | `tauri-plugin-window-state`               | remembers size and position                                   |
| One window only     | `tauri-plugin-single-instance`            | second launch focuses the first                               |
| Updates             | `tauri-plugin-updater` + `-process`       | signed in-app updates                                         |
| Errors              | `thiserror`                               | one typed error enum per crate                                |
| Data                | `serde`, `indexmap`, `regex`, `chrono`    | models, ordered maps, patterns, dates                         |
| Tests               | `insta`, `tempfile`                       | snapshot tests, temp folders                                  |
| Frontend            | React, TypeScript, Vite, Tailwind, shadcn/ui (Radix) | accessible components out of the box                |
| PDF viewer          | `pdfjs-dist`                              | same viewer behavior on WebView2 and WebKitGTK                |
| Frontend tests      | `vitest`, Testing Library, `axe-core`     | logic, components, accessibility                              |

Exact APIs and versions are checked against current documentation at the start
of the phase that uses them.

Viewer note: because the PDFs use non-embedded standard fonts, pdf.js must be
given its bundled standard-font files; they ship inside the app. Everything
works offline.

### 3.7 Security

- Minimal capabilities: the frontend gets no shell and no general filesystem access.
- `read_pdf` only serves files inside output folders of the current session or the history.
- Strict CSP, no remote content. Updates are signature-verified.

---

## 4. Installation

Goal: one line to paste, no questions asked, no administrator rights.

### Windows 11

```powershell
irm https://<install-host>/install.ps1 | iex
```

The script: finds the latest release, downloads the installer, verifies its
SHA-256, runs it silently as a per-user install, creates Start-menu and desktop
shortcuts, and opens the app. WebView2 is already part of Windows 11; the
installer includes the bootstrapper as a fallback.

### Linux

```bash
curl -fsSL https://<install-host>/install.sh | sh
```

The script detects the system and picks the right package:

| System                 | Action                                                                  |
| ---------------------- | ----------------------------------------------------------------------- |
| Debian / Ubuntu / Mint | downloads the `.deb`, installs it with `apt` (dependencies resolved)     |
| Fedora / openSUSE      | downloads the `.rpm`, installs it with the system package manager        |
| Anything else (Arch…)  | installs the AppImage under `~/.local/bin` with a menu entry and icon    |

Both scripts are short, readable, checksum-verified, and have an uninstall
counterpart. Direct downloads of the installers stay available on the release
page for people who prefer double-clicking.

After the first install, the app updates itself (signed updates, one
"Actualizar ahora" button).

Requirement: the release files and scripts must be publicly downloadable
(see decision 4 in section 13).

---

## 5. UI guidelines applied

Sources: Nielsen's 10 usability heuristics, WCAG 2.2 AA, Microsoft Fluent 2
(Windows 11) and GNOME HIG for native conventions, plus established guidance
for older and low-confidence users.

| Rule                                   | Concrete application                                                                                   |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Readable text                          | Base 17 px, never below 14 px; text-size setting (Normal / Grande / Muy grande)                         |
| Large targets                          | Every clickable control at least 44 × 44 px, generous spacing                                           |
| Contrast                               | Body text ≥ 7:1, everything else ≥ 4.5:1; **light theme by default**, dark optional                     |
| One primary action per screen          | One filled button, always bottom-right; secondary actions are outlined                                  |
| Visible system status                  | Step indicator "Paso 2 de 3"; real progress "12 de 27 — MARTÍNEZ, Ana"                                  |
| Words, not icons alone                 | Every icon has a text label; no icon-only buttons                                                       |
| Match the user's language              | "hoja", "carpeta", "boletín", "notas"; never "sheet", "workspace", "pipeline"                           |
| Recognition over recall                | Recent files, remembered folder, suggested folder name, summary of what will happen before generating   |
| Error prevention                       | Only supported files selectable; "Generar" disabled with the reason shown; overwrite asked before, not after |
| Errors people can act on               | "What happened" + "what to do" in plain Spanish; technical detail behind "Ver detalles" with a copy button |
| User control                           | "Atrás" on every step without losing choices; "Cancelar" during generation                              |
| No hidden gestures                     | Drag and drop always paired with a visible button                                                       |
| Native where it builds trust           | OS file/folder pickers, OS title bar, OS scaling respected                                              |
| Keyboard and focus                     | Enter = primary action, Esc = back/close, visible focus ring, logical tab order                         |
| Calm motion                            | Short transitions only; honors "reduce motion"                                                          |
| Consistency                            | One design-token set (color, spacing, radius, type scale) used everywhere                               |

### 5.1 Color palette: blue-green (teal)

The interface uses teal tones. These colors apply to the **app interface
only**; the colors inside the PDFs do not change.

| Token              | Light theme | Dark theme | Use                                         |
| ------------------ | ----------- | ---------- | ------------------------------------------- |
| `primary`          | `#0F766E`   | `#2DD4BF`  | primary button, active step, links          |
| `primary-hover`    | `#115E59`   | `#5EEAD4`  | hover / pressed                             |
| `on-primary`       | `#FFFFFF`   | `#042F2E`  | text on primary                             |
| `accent`           | `#0E7490`   | `#22D3EE`  | secondary highlights, selected rows         |
| `background`       | `#F3FAF9`   | `#0B1F22`  | window background                           |
| `surface`          | `#FFFFFF`   | `#12292D`  | cards, dialogs                              |
| `surface-muted`    | `#E3F2F1`   | `#183539`  | drop area, table header, hover rows         |
| `border`           | `#B9D8D6`   | `#2A4D52`  | borders, dividers                           |
| `text`             | `#0F2A2E`   | `#E8F5F4`  | body text                                   |
| `text-muted`       | `#44646A`   | `#9FC2C1`  | secondary text                              |
| `focus-ring`       | `#0E7490`   | `#67E8F9`  | keyboard focus outline (3 px)               |
| `success`          | `#15803D`   | `#4ADE80`  | done states                                 |
| `warning`          | `#B45309`   | `#FBBF24`  | notices                                     |
| `danger`           | `#B91C1C`   | `#F87171`  | errors, destructive actions                 |

Rules: status is never conveyed by color alone (always icon + text); every
text/background pair is checked automatically against the contrast targets
above, and a token is adjusted if a pair fails.

Measured result (Phase 6): every pair passes with the values above, so no
token was changed. One constraint follows from the measurements: `success` and
`warning` text must not sit on `surface-muted` (4.36:1 in the light theme);
notices therefore use a `surface` background with a colored border and icon.
The danger button is outlined, not filled, since there is no `on-danger` token.

---

## 6. Screens and flow

One window, a three-step assistant, then results.
Minimum size 1024 × 700; opens at the remembered size, never forced fullscreen.

### 6.0 Inicio

- Large drop area: "Arrastra aquí tu archivo de notas" with the button **"Elegir archivo de notas"**.
- "Archivos recientes" (last 5, with date; missing files greyed with an explanation).
- "Boletines anteriores" (history).
- Discreet "Ajustes" and "Ayuda" buttons.

### 6.1 Paso 1 — Revisar

- Several usable sheets: one large card per sheet with its name and student count. Only one: preselected, choice skipped.
- Summary card: Materia, Grupo, Profesor, Periodo (as read from the file).
- Student table: number, name, code, absences, final grade. Checkboxes, all selected by default, "Seleccionar todos / ninguno", search box.
- Yellow notice bar for warnings in plain words (e.g. "La numeración de 3 estudiantes estaba incorrecta en el archivo y se corrigió automáticamente").
- **"Ver ejemplo"**: generates one report to a temp folder and shows it in the viewer.
- Primary: **"Continuar"**.

### 6.2 Paso 2 — Guardar

- "Los boletines se guardarán en:" with the path and a **"Cambiar carpeta…"** button (native picker).
- Suggested folder, created automatically: `Documentos/Boletines/<Materia> – Grupo <n> – <Periodo>/`.
- Checkbox "Recordar esta carpeta base".
- "Fecha del boletín": today by default, with a date picker.
- Folder already has report cards: clear choice **"Reemplazar"** / **"Guardar en una carpeta nueva"** before anything is written.
- Summary sentence: "Se crearán 27 boletines en …".
- Primary: **"Generar 27 boletines"**.

### 6.3 Generando

- Progress bar with real count and current student name; **"Cancelar"**.
- On cancel: reports already created are kept and listed.
- On failure for one student: generation continues; the failure is listed at the end with a retry button.

### 6.4 Resultados

- Left: list of generated reports (number + student name), searchable.
- Right: **built-in PDF viewer** — zoom, "Ajustar al ancho", page navigation, "Anterior / Siguiente estudiante" (also arrow keys).
- Actions: **"Abrir carpeta"**, "Abrir con otro programa", "Imprimir", **"Generar otros boletines"**.

### 6.5 Ajustes

Default base folder, text size, theme (Claro / Oscuro / Según el sistema),
updates, "Abrir carpeta de registros", app version. Nothing here is required
for normal use.

### 6.6 Ayuda

A short illustrated page: the three steps, supported file formats, and what to
do if a file is not recognized.

---

## 7. Features (new vs. today)

| #    | Feature                                                                   | Today                                  |
| ---- | ------------------------------------------------------------------------- | -------------------------------------- |
| 7.1  | Built-in PDF viewer with per-student navigation                           | none                                   |
| 7.2  | Review of students and metadata before generating                         | none (blind generation)                |
| 7.3  | One-report preview ("Ver ejemplo")                                        | none                                   |
| 7.4  | Smart output folder: remembered, auto-named, overwrite protection         | forced workspace prompt on first run   |
| 7.5  | History of past generations, reopenable in the viewer                     | none                                   |
| 7.6  | Recent files, drag and drop                                               | none                                   |
| 7.7  | Real progress, cancel, window never freezes                               | UI blocks, bar does not animate        |
| 7.8  | Choose which students to generate                                         | all or nothing                         |
| 7.9  | Choose the date printed on the report                                     | always "now"                           |
| 7.10 | Errors in plain Spanish with copyable detail; errors reach the log        | errors lost in a `print()`             |
| 7.11 | Sheet cards with student counts; auto-skip when there is only one         | dropdown dialog                        |
| 7.12 | One-line install, per-user, no admin; in-app updates                      | loose executable, manual download      |
| 7.13 | Remembered window size, single instance, text-size and theme settings     | fixed 1920×1080 / forced maximized     |
| 7.14 | Fast start (single native binary)                                         | slow self-extracting Python bundle     |
| 7.15 | After 1.0: "Unir todos en un solo PDF" to print a whole class at once     | none                                   |

Migration: on first run, read `~/.mocha_edu_card_reporter/workspace_path.txt`
if present and use it as the default base folder.

---

## 8. Code quality standards

- **Engine crate has no UI and no Tauri dependency**; it is usable and testable from the command line (`mocha-cli`).
- **Rust:** `clippy` with warnings as errors, `rustfmt`, no `unwrap` outside tests, typed errors with `thiserror`, doc comments on every public item.
- **TypeScript:** `strict` mode, ESLint, Prettier; IPC types generated, never hand-written.
- **Error codes, not strings:** `FILE_NOT_FOUND`, `FILE_LOCKED`, `UNSUPPORTED_FORMAT`, `NO_GRADEBOOK_FOUND`, `EMPTY_SHEET`, `OUTPUT_NOT_WRITABLE`, `PDF_FAILED`, `CANCELLED`, `INTERNAL`. Each maps to one Spanish message with a suggested action.
- **All UI strings** in one Spanish dictionary file.
- **Tests:** parity tests (section 2), unit tests per helper, command tests, component and accessibility tests.
- **CI on every push:** lint + tests on Linux and Windows.

---

## 9. Step-by-step phases

Each phase ends in a verifiable state. The engine is ported and proven
**before** any interface work, ordered by risk.

### Phase 0 — Oracle and corpus ✅

1. The current state is already tagged `v0.2.0`; create branch `redo/tauri`.
2. `git mv` the Python app into `legacy/` and `templates/` into `fixtures/`; adjust only the paths in the legacy test; disable the old release workflow.
3. Add `tools/oracle`: a script that runs the legacy engine and dumps checkpoints A, B, C as JSON and the PDFs for every file and sheet in the corpus, into `fixtures/expected/`.
4. Build `tools/pdfdiff`: render two PDFs to images and compare; compare extracted text and its positions.
5. Collect the owner's real gradebooks into `corpus-private/`.
6. Commit the expected results for the public fixtures.

**Done when:** one command regenerates all expected results, and the existing Python tests still pass.

**Result:** `fixtures/expected/` has 13 cases (manifest.json, per-case JSON + PDFs). 232 students, 12 OK cases + 1 corrupted. `tools/pdfdiff/pdfdiff.py` operational. Legacy pytest: 10 passed. Step 5 (corpus-private) pending user.

### Phase 1 — PDF spike (highest risk first) ✅

1. Cargo workspace with `mocha-engine` and `mocha-cli`.
2. Hard-code the data of two real students (one short, one long enough to wrap labels) and produce their PDFs with the Rust layout module.
3. Compare against the oracle PDFs with `pdfdiff`.

**Done when:** both PDFs meet checkpoint D. If not achievable, switch crate (fallback in 3.4) before continuing. This is a go / no-go gate.

**Result:** GO with `pdf-writer 0.15`. 232/232 PDFs match oracle (max position delta 4.7e-05 pt, pixel diff 0%).

### Phase 2 — Reading and normalizing ✅

1. `pycompat.rs`, `patterns.rs`, `text.rs` with unit tests against Python outputs.
2. `.csv`, `.xlsx`, `.xls` readers and `normalize.rs`.
3. Sheet detection (`get_sheet_names`).

**Done when:** checkpoint A matches for the whole corpus, and the sheet lists match.

**Result:** All modules implemented. 22,994 pycompat vectors pass. Checkpoint A matches for all 13 cases + 347 synthetic cases.

### Phase 3 — Parser ✅

1. Tabular layout (2019 / 2025), label extraction, numbering correction and its warnings.
2. Carnet layout.
3. Metadata extraction helpers.

**Done when:** checkpoints B and C match for the whole corpus, including the "broken numbering" test case.

**Result:** Checkpoints B and C match for all 13 cases + 23 synthetic parser cases.

### Phase 4 — PDF generator, complete ✅

1. Finish paragraph, table and document modules; page breaks with repeated header.
2. Report builder and styles; file naming.
3. Processor: convert → parse → generate, per-student callbacks, cancellation.

**Done when:** checkpoint D matches for every student of every file in the corpus, and `mocha-cli` generates a full class end to end. **The engine is now complete and proven without any UI.**

**Result:** 232/232 PDFs match. `cargo clippy` clean, 41 engine tests pass (unit 16, checkpoint_a 2, checkpoint_b 5, end_to_end 12, pdf_spike 5, pycompat_vectors 1). `tools/parity/check.py` → "all checks match". See `PARITY_NOTES.md`.

### Phase 5 — App shell ✅

1. Scaffold Tauri v2 + React + TypeScript + Vite; identifier, icons, window.
2. Commands from 3.5 with generated TypeScript bindings; progress channel; cancel.
3. Settings and history stores; path validation for `read_pdf`; capabilities and CSP.

**Done when:** a bare debug page can pick a file, list students and generate PDFs.

**Result:** `src-tauri/` complete. Tauri 2.12.5 with plugins (dialog, opener, store, log, window-state, single-instance). 35 Rust tests pass, clippy clean. Bindings generated. Identifier `com.mochapulse.educardreporter` (provisional). Not verified: real window run, bundles, Windows paths.

### Phase 6 — Design system ✅

1. Tokens: color (light + dark), type scale, spacing, radius, focus ring.
2. Base components: Button, Card, Stepper, Table, Notice, Dialog, Progress, EmptyState, PathField.
3. Spanish dictionary; error-code → message table.
4. Automated accessibility checks on a component gallery.

**Done when:** the gallery passes the checks at all three text sizes in both themes.

**Result:** `src/styles/tokens.css|ts`, `src/components/ui/*` (15 components), `src/i18n/es.ts` + `errors.ts`, `src/gallery/`, contrast tests. 164 frontend tests pass. Typecheck, lint, format, build all pass. Gallery at `?gallery`.

### Phase 7 — Assistant screens ✅

1. Inicio, Paso 1, Paso 2, Generando.
2. Flow state machine (back/forward without losing choices, cancel, error states).
3. Folder suggestion, overwrite check, date picker.

**Done when:** the full flow works with the mouse only, and again with the keyboard only.

**Result:** 7 screens implemented: home, review (sheet + students steps), save, generating, results, settings, help. Flow state machine in `src/features/assistant/state.ts` (457 lines, 21 action types). `useAssistant` hook connects reducer to API. Mock API + dev scenarios (`?demo=<name>`). Keyboard: Esc = back, Enter = primary action. Screenshots in `target/screenshots/`.

### Phase 8 — Viewer and results ✅

1. pdf.js viewer: zoom, fit width, pages, lazy rendering, bundled standard fonts.
2. Results screen with student navigation; "Ver ejemplo" reuses the viewer.
3. "Abrir carpeta", "Abrir con otro programa", "Imprimir".

**Done when:** all corpus PDFs display correctly on Windows 11 and Linux.

**Result:** `src/components/pdf-viewer/` (14 files). pdf.js legacy build, bundled worker, Liberation Sans fonts in `public/pdfjs/standard_fonts/`. CSP-safe (no eval, no wasm, no remote URLs). Integrated in results-screen with prev/next navigation, search, zoom. Tested in headless Chromium 155 with no CSP violations. Not verified: WebKitGTK, WebView2.

### Phase 9 — Quality of life ✅

Recent files, history, settings, help page, window state, single instance,
migration of the old workspace path, error detail dialog.

**Done when:** closing and reopening the app restores folder, window and history.

**Result:** All implemented in backend (src-tauri: recent files, history with persistence, settings store, window-state plugin, single-instance plugin, legacy workspace migration). Frontend: settings screen (base folder, text size, theme, logs), help screen (steps, formats, troubleshooting), error panels with technical details. Install scripts in `install/`.

### Phase 10 — Packaging, install scripts, updates, CI

1. Windows: NSIS installer, per-user. Linux: `.deb`, `.rpm`, AppImage.
2. `install.ps1`, `install.sh` and their uninstall counterparts, with checksum verification.
3. Updater: signing keys, update feed published with each release.
4. `ci.yml` (lint + tests, both systems) and `release.yml` (native build on each system, installers + checksums + update feed attached to the release).
5. Windows code signing wired into the release job when a certificate is available.
6. Test the one-line install on a clean Windows 11 machine, a clean Ubuntu and a clean Arch.

**Done when:** on clean machines, the one-line command installs the app and it generates PDFs.

**Result (Linux):** `pnpm tauri build` produces `MochaEduCardReporter_1.0.0_amd64.deb` (9.23 MiB), `MochaEduCardReporter-1.0.0-1.x86_64.rpm` (9.24 MiB) and `MochaEduCardReporter_1.0.0_amd64.AppImage` (107.55 MiB), each with an updater `.sig`. Verified: the `.deb` installs `/usr/bin/mocha-edu-card-reporter`, the desktop entry and the icons; the `.rpm` payload was listed and contains the same; the AppImage opens a real window. The production binary was also smoke-tested outside any bundle: it renders under the strict CSP with the embedded `dist`, and a second launch leaves exactly one window and one process (single-instance).

**Updater:** `tauri-plugin-updater 2.14.1` + `tauri-plugin-process 2.4.0` are registered, `createUpdaterArtifacts` is on, the public key lives in `src-tauri/tauri.conf.json` and the feed is `https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/latest.json`. The app never checks at startup: only Ajustes → Actualizaciones does, on request. The private key is `~/.tauri/mocha-edu-card-reporter.key` (empty password) and must become the `TAURI_SIGNING_PRIVATE_KEY` repository secret; it is not in the repository.

**CI/CD:** `.github/workflows/ci.yml` runs formatting, lint, typecheck and tests for the frontend plus `fmt`/`clippy -D warnings`/`test` for the Rust workspace on `ubuntu-22.04` and `windows-latest` on every push. `.github/workflows/release.yml` builds both platforms with `tauri-action`, signs the updater artifacts, attaches the installers as a **draft** release (publishing stays manual; `/releases/latest` ignores drafts), optionally imports a Windows certificate, and publishes `SHA256SUMS` plus the install scripts.

**Install scripts:** `install/install.sh` and `install/install.ps1` point at `mochapulse/MochaEduCardReporter`, verify the checksum, and warn when AppImage support is missing (`libfuse2`). `install/README.md` documents the one-liners.

**Not verified:** the clean-machine installs (item 6) and the whole Windows side — no Windows machine, VM or clean Ubuntu/Arch host was available. The bundle names were checked against the patterns the scripts match.

**Defect found on the first real run:** `src/app/dev-scenarios.ts` treated a missing `?demo` query as `home`, so `pnpm tauri dev` always swapped in the mock API and never touched the real backend. It now returns `null` without the parameter, covered by `src/app/dev-scenarios.test.ts`.

### Phase 11 — Validation and cleanup

1. Session with the teacher on a real gradebook: observe without helping, note every hesitation, fix, repeat once.
2. Final parity run over the whole corpus with the release build.
3. Decide whether `legacy/` is deleted or kept as a regression reference; rewrite `README.md`; tag `v1.0.0`.

**Done when:** the teacher completes the flow alone, and the final parity run is clean.

**Result (partial):** the final parity run used the release binary (`legacy/.venv/bin/python tools/parity/check.py`): 13 cases, 232 students, all four checkpoints match. `legacy/` stays in the repository as the dev-only parity oracle, never shipped and never imported by the app; `README.md` was rewritten for the native app and documents that split. The teacher session and the `v1.0.0` tag are still open — see below.

### Remaining work (human steps only)

Everything a machine could do on this host is done. What is left needs a person, another operating system, or the owner's decision:

1. **Teacher session** (Phase 11 item 1): observe a real gradebook session without helping, note every hesitation, fix, repeat once. Blocks item 3.
2. **Tag `v1.0.0`** (Phase 11 item 3): deliberately not created; it should follow the teacher session.
3. **Windows:** build and test the app, the NSIS bundle and `install.ps1` on a real Windows 11 machine. Until then the Windows half of `ci.yml` and `release.yml` is unproven.
4. **Clean-machine installs** (Phase 10 item 6): one-line install on a clean Windows 11, a clean Ubuntu and a clean Arch.
5. **Interactive GUI walkthrough:** the native file picker, drag-and-drop and the real PDF in WebKitGTK were not driven end to end (window automation on Hyprland 0.56 was unreliable). The backend paths are covered by `src-tauri/src/tests.rs`, which runs the real engine: inspect, generate 27 PDFs, cancel, preview, `read_pdf` authorization, history, settings.
6. **Repository secret:** add `TAURI_SIGNING_PRIVATE_KEY` (contents of `~/.tauri/mocha-edu-card-reporter.key`, empty password) before the first release run.
7. **Open decisions:** whether the "NOTE DÉFINITIVE" label stays undrawn (current behavior reproduced); Windows code-signing certificate (optional for 1.0, needs `bundle.windows.certificateThumbprint` plus `WINDOWS_CERTIFICATE`/`WINDOWS_CERTIFICATE_PASSWORD`); real gradebooks in the git-ignored `corpus-private/`.

### Current verification status

| Check                                   | Status |
| --------------------------------------- | ------ |
| `cargo fmt --all --check`               | ✅ clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ clean |
| 41 engine tests (unit, A, B, e2e, spike, pycompat) | ✅ pass |
| 35 tauri tests (real engine)            | ✅ pass |
| `pnpm typecheck`                        | ✅ clean |
| `pnpm lint`                             | ✅ clean |
| `pnpm format:check`                     | ✅ clean |
| 220 frontend tests                      | ✅ pass |
| `pnpm build`                            | ✅ pass |
| Parity check with the release build (13 cases, 232 students) | ✅ all match |
| `pnpm tauri dev`, real window           | ✅ run; one defect found and fixed |
| `pnpm tauri build` (deb, rpm, AppImage) | ✅ built and inspected |
| Production binary under strict CSP      | ✅ renders correctly |
| Single instance                         | ✅ second launch leaves one process |
| Updater end to end                      | ⬜ not exercised (needs a published release) |
| Windows build, NSIS, `install.ps1`      | ⬜ not tested |
| Clean Ubuntu / Arch / Win11 installs    | ⬜ not tested |
| Interactive GUI walkthrough             | ⬜ not driven end to end |
| Teacher session, tag `v1.0.0`           | ⬜ pending (human) |

---

## 10. Risks

| Risk                                                              | Mitigation                                                                                       |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| PDF layout does not match ReportLab closely enough                | Spike and go / no-go gate in Phase 1; automated visual and positional diff on every student       |
| pandas round-trip rules are subtler than listed                   | Checkpoint A compares the full grid on the whole corpus; rules derived from evidence, not assumed |
| A real-world file behaves differently from every tested file      | Private corpus of real gradebooks; `mocha-cli diff` to check any new file against the oracle      |
| `.xls` reading differs between calamine and xlrd                  | Covered by checkpoint A; old-format fixtures already in the repo                                  |
| Python vs. Rust regex / number-format differences                 | `pycompat` helpers, each tested against Python output                                             |
| Unsigned installer flagged by SmartScreen or Defender             | Code signing when a certificate is available; checksum-verified installer meanwhile               |
| Printing from the webview unreliable on Linux                     | "Imprimir" falls back to the system PDF viewer                                                    |
| Spreadsheet open in Excel while reading (locked file)             | Dedicated message: "Cierra el archivo en Excel e inténtalo de nuevo"                              |
| Accents, spaces, long paths, OneDrive folders                     | Fixtures with such names; paths never pass through a shell                                        |
| Linux distro differences                                          | Three package formats; tested on Ubuntu LTS and Arch                                              |

---

## 11. Honest cost of this revision

Compared with revision 1 (Python engine wrapped, untouched), porting the engine:

- **Gains:** one self-contained binary, instant start, small installer, simpler install and updates, one language for the whole backend, cleaner code.
- **Costs:** about 1,300 lines of proven logic are rewritten, plus a ReportLab-compatible layout module. Parity is no longer guaranteed by "we did not touch it" but by tests, so its strength equals the breadth of the corpus. Phases 0–4 exist entirely to pay this cost safely.

---

## 12. Out of scope

- Any change to how gradebooks are interpreted or how the PDF looks.
- Editing grades inside the app.
- Emailing report cards.
- macOS builds.
- Cloud sync or accounts.

---

## 13. Decisions to confirm before Phase 0

Recommended defaults are stated; work proceeds on them unless changed.

1. **Frontend stack:** React + TypeScript + Tailwind + shadcn/ui.
2. **Same repository**, branch `redo/tauri`; the old Python app lives in `legacy/`. *(confirmed; kept after 1.0 as the parity oracle)*
3. **Version:** the rewrite ships as `1.0.0`.
4. **Install host:** GitHub Releases of a public repository, with the scripts served from the same repository. *(confirmed: `mochapulse/MochaEduCardReporter`, public)*
5. **Default output location:** `Documentos/Boletines/`, replacing the forced "workspace" prompt.
6. **Real gradebooks for the corpus:** the owner provides them in a private, git-ignored folder.
7. **Windows code-signing certificate:** optional for 1.0; without it Windows may show a warning on first install.
8. **Merged PDF for printing:** after 1.0.
