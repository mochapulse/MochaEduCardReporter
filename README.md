# MochaEduCardReporter

A desktop application that reads Excel or CSV gradebooks and generates student report-card PDFs.

## Requirements

- Python 3.10+
- Dependencies listed in `requirements.txt`
- On Linux, Python must include Tkinter support. If needed, install your distro package such as `python3-tk`.

## Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/dramirezbe/MochaEduCardReporter.git
   cd MochaEduCardReporter
   ```

2. Create and activate a virtual environment:
   ```bash
   python -m venv venv
   # Linux/macOS
   source venv/bin/activate
   # Windows
   venv\Scripts\activate
   ```

3. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```

4. Configure your environment variables in `.env` if you need to override defaults.

5. Run the application:
   ```bash
   python src/app.py
   ```

## Testing

The test suite generates PDFs for every supported `.xls`, `.xlsx`, and `.csv` fixture in `templates/`.

```bash
python -m pytest
```

## Compilation Flow

The build uses `app.spec` as the single source of truth for both Windows and Linux. Build on the same OS you want to distribute for; PyInstaller does not cross-compile Windows executables from Linux or Linux binaries from Windows.

The bundled executable includes `.env.example`. On first run, the app creates its runtime config in the user profile:

- Linux: `~/.mocha_edu_card_reporter/.env`
- Windows: `%USERPROFILE%\.mocha_edu_card_reporter\.env`

The user will also be asked to choose a workspace folder on first run. Generated PDFs are saved wherever the user chooses in the app.

The Windows executable metadata is configured in `app.spec`:

- App icon: `assets/app.ico`
- Version resource: `resources/windows_version_info.txt`

If you change the icon design, regenerate it with:

```bash
python scripts/generate-icon.py
```

### Recommended Scripted Build

#### Linux

From the repository root:

```bash
./scripts/build-linux.sh
```

Output:

```bash
dist/MochaEduCardReporter
```

Run it:

```bash
./dist/MochaEduCardReporter
```

#### Windows

Open PowerShell in the repository root.

Run PowerShell as **Administrator** for deploy/build steps to avoid permission-denied failures.

If script execution is blocked for the current shell session, run:

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
```

Build:

```powershell
.\scripts\build-windows.ps1
```

Output:

```powershell
dist\MochaEduCardReporter.exe
```

Run it:

```powershell
.\dist\MochaEduCardReporter.exe
```

### Manual Build Commands

Use these if you do not want to use the helper scripts.

#### Linux

```bash
python -m venv venv
source venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
python -m pytest
python -m PyInstaller --clean --noconfirm app.spec
```

#### Windows

```powershell
py -m venv venv
.\venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
python -m pytest
python -m PyInstaller --clean --noconfirm app.spec
```

## Build Checklist

- Run `python -m pytest` before compiling.
- Build from a clean virtual environment when preparing a release.
- Compile on each target OS separately.
- Smoke test the generated executable by loading one workbook from `templates/` and generating PDFs into a temporary folder.
- If you need to change defaults, edit `.env.example` before building or edit the generated runtime `.env` after first launch.

## Windows Signing From Linux

PyInstaller does not cross-compile Windows executables from Linux. Build `dist\MochaEduCardReporter.exe` on Windows, in a Windows VM, or in a Windows CI job. After you have the `.exe`, you can sign it from Linux.

Install the signing tool:

```bash
sudo apt install osslsigncode
```

You need a real code-signing certificate from a certificate authority, usually exported as `.pfx` or `.p12`. A self-signed certificate is useful for local tests, but it will not improve Microsoft SmartScreen trust for users.

Sign the executable:

```bash
export WINDOWS_SIGN_CERT=/path/to/certificate.pfx
export WINDOWS_SIGN_PASSWORD='certificate-password'
./scripts/sign-windows-from-linux.sh dist/MochaEduCardReporter.exe
```

Output:

```bash
dist/MochaEduCardReporter-signed.exe
```

For strongest SmartScreen reputation, use an EV code-signing certificate or build reputation over time with a standard OV certificate and consistent signed releases.
