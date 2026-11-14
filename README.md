# MochaEduCardReporter

Turns a teacher's gradebook (Excel `.xlsx` / `.xls`, or `.csv`) into one PDF
report card per student.

It is a single native desktop application — Tauri v2 with a Rust engine and a
React interface. There is no Python, no second process and no runtime to
install. The PDFs are identical, page for page and line for line, to the ones
the previous Python version produced (see [Parity](#parity)).

- **Windows 11** (WebView2 is part of Windows 11)
- **Linux x86_64** (WebKitGTK 4.1)

The interface is in Spanish; the report cards are in French.

## Install

Windows 11, in PowerShell:

```powershell
irm https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/install.ps1 | iex
```

Linux:

```bash
curl -fsSL https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/install.sh | sh
```

No administrator rights on Windows (per-user install). On Linux the script
picks the package for the distribution: `.deb` on Debian/Ubuntu/Mint, `.rpm`
on Fedora/RHEL/openSUSE, and the AppImage everywhere else (no root needed).
Every download is checked against `SHA256SUMS` before anything is installed.

The installers can also be downloaded directly from the
[releases page](https://github.com/mochapulse/MochaEduCardReporter/releases).

To remove the program (documents and generated PDFs are never touched):

```powershell
irm https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/uninstall.ps1 | iex   # Windows
```

```bash
curl -fsSL https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/uninstall.sh | sh
```

## Use

1. **Revisar** — choose the gradebook, pick the sheet when the file has
   several, check the students and the data read from the file.
2. **Guardar** — choose where the PDFs go (a suggested folder is offered) and
   the date printed on them.
3. **Generar** — watch real progress, cancel if needed, and read the result in
   the built-in PDF viewer.

The app remembers the last files, the last output folder and the past
generations; nothing has to be configured to get work done. Updates are
checked from **Ajustes → Actualizaciones**; they are signed and verified
before installing.

## Development

Requirements: Rust (stable), Node 22+, pnpm 12, and on Linux the Tauri build
dependencies:

```bash
sudo apt-get install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

```bash
pnpm install
pnpm tauri dev      # the real app, real backend, real window
pnpm dev            # only the interface, in a browser, with the in-memory mock API
```

`?demo=<name>` in `pnpm dev` opens a screen directly with fixture data
(`home`, `sheets`, `review`, `save`, `generating`, `results`, `failures`,
`settings`, `help`, …); without it, development uses the real backend.

Checks (all green before a release):

```bash
pnpm typecheck && pnpm lint && pnpm format:check && pnpm test && pnpm build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Building the installers (updater artifacts are signed with the private key):

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/mocha-edu-card-reporter.key)"
pnpm tauri build
```

Output lands in `target/release/bundle/` (`deb/`, `rpm/`, `appimage/`,
`nsis/`) and `target/release/bundle/*/*.sig`.

### Layout

```
crates/mocha-engine/   pure Rust engine: read → normalize → parse → PDF
crates/mocha-cli/      development tool: dumps every checkpoint, diffs a file
src-tauri/             app shell: typed commands, settings, history, updater
src/                   React interface (Spanish dictionary in src/i18n/es.ts)
fixtures/              public spreadsheets and the recorded Python output
corpus-private/        real gradebooks, git-ignored
tools/oracle/          runs the Python engine and records the checkpoints
tools/parity/          compares the Rust engine against that recording
tools/pdfdiff/         renders two PDFs and compares text, positions and pixels
legacy/                the previous Python app, unedited: the parity oracle
install/               install/uninstall scripts for Windows and Linux
```

## Parity

The Python engine in `legacy/` is the specification. The Rust engine must
produce the same result at four checkpoints: the normalized cell grid, the
parsed metadata, the rows built per student, and the PDF itself (same text,
fonts, positions within 0.5 pt, colors and lines).

```bash
python3 -m venv legacy/.venv
legacy/.venv/bin/pip install -r legacy/requirements.txt
legacy/.venv/bin/python tools/parity/check.py            # whole corpus
legacy/.venv/bin/python tools/parity/check.py --skip-pdf # faster
```

The public fixtures plus whatever real gradebooks are placed in
`corpus-private/` are checked. Known and preserved oddities of the Python
engine are listed in `PARITY_NOTES.md`; changing one is a separate decision.

## Releases

Pushing a `v*` tag runs `.github/workflows/release.yml`: it builds on
Ubuntu 22.04 and Windows, signs the updater artifacts, creates a **draft**
release with every installer, `latest.json`, the signatures and `SHA256SUMS`,
and attaches the install scripts. Publishing the draft is a human decision —
the one-line installers read `/releases/latest`, which ignores drafts.

Secrets used by the workflow:

| Secret | Required | Purpose |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | yes | Signs updater artifacts. Content of `~/.tauri/mocha-edu-card-reporter.key`; the public key is in `src-tauri/tauri.conf.json`. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | only if the key has one | Password of that private key. |
| `WINDOWS_CERTIFICATE` / `WINDOWS_CERTIFICATE_PASSWORD` | optional | Base64 PFX to sign the Windows installer. Also set `bundle.windows.certificateThumbprint` in `src-tauri/tauri.conf.json` once the certificate exists. |

Losing the private key means installed copies can no longer verify updates:
back it up, never commit it.

## Licence

Internal project for the Modern Languages department; distribution handled by
its maintainer.
