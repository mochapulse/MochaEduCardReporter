# CoffeeEduMailer

A desktop application that reads Excel or CSV gradebooks and generates student report-card PDFs.

## Requirements

- Python 3.10+
- Dependencies listed in `requirements.txt`
- On Linux, Python must include Tkinter support. If needed, install your distro package such as `python3-tk`.

## Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/dramirezbe/CoffeeEduMailer.git
   cd CoffeeEduMailer
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

- Linux: `~/.coffee_edu_mailer/.env`
- Windows: `%USERPROFILE%\.coffee_edu_mailer\.env`

The user will also be asked to choose a workspace folder on first run. Generated PDFs are saved wherever the user chooses in the app.

### Recommended Scripted Build

#### Linux

From the repository root:

```bash
./scripts/build-linux.sh
```

Output:

```bash
dist/CoffeeEduMailer
```

Run it:

```bash
./dist/CoffeeEduMailer
```

#### Windows

Open PowerShell in the repository root.

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
dist\CoffeeEduMailer.exe
```

Run it:

```powershell
.\dist\CoffeeEduMailer.exe
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
