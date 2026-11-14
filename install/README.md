# Installing MochaEduCardReporter

No administrator rights on Windows. On Linux, a system package uses `sudo`
(you are asked for your password); without `sudo` the AppImage is installed.
The scripts are published with every release of
[`mochapulse/MochaEduCardReporter`](https://github.com/mochapulse/MochaEduCardReporter).

## Windows 11

```powershell
irm https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/install.ps1 | iex
```

1. Checks 64-bit Windows 10/11 and TLS 1.2+.
2. Reads the latest release from the GitHub API.
3. Downloads the `*-setup.exe` installer and `SHA256SUMS`.
4. Checks the installer's SHA-256 (aborts on mismatch).
5. Runs it silently (`/S`, per user), checks the exit code, opens the app.

## Linux (x86_64)

```bash
curl -fsSL https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/install.sh | sh
```

1. Checks Linux x86_64, `curl` (or `wget`) and `sha256sum`.
2. Reads the latest release; downloads `SHA256SUMS` and the package.
3. Checks the SHA-256 before any install step (aborts on mismatch).
4. Installs by distribution: Debian/Ubuntu `.deb` via `apt`; Fedora/RHEL/
   openSUSE `.rpm` via `dnf` or `zypper`; anything else the AppImage.
5. AppImage: copies it to `~/.local/share/MochaEduCardReporter/`, links
   `~/.local/bin/mocha-edu-card-reporter`, adds a menu entry.

## Environment variables

- `MOCHA_REPO=OWNER/REPO`: repository to read releases from.
- `MOCHA_VERSION=vX.Y.Z`: install this tag instead of the latest release.
- `MOCHA_FORCE_APPIMAGE=1`: Linux only, always use the AppImage.

## Verifying a download by hand

```bash
sha256sum MochaEduCardReporter_1.0.0_amd64.deb
grep MochaEduCardReporter_1.0.0_amd64.deb SHA256SUMS
```

```powershell
Get-FileHash .\MochaEduCardReporter_1.0.0_x64-setup.exe -Algorithm SHA256
```
(The hash must equal the matching line in `SHA256SUMS`.)

## Uninstalling

```powershell
irm https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/uninstall.ps1 | iex       # Windows
```

```bash
curl -fsSL https://github.com/mochapulse/MochaEduCardReporter/releases/latest/download/uninstall.sh | sh  # Linux
```

These remove the program only. Documents and generated PDFs are never touched.
