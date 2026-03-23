# CoffeeEduMailer

A desktop application that reads an Excel file with student grades, generates report cards, and sends them automatically by email.

## Requirements

- Python 3.10+
- Dependencies listed in `requirements.txt`

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

4. Configure your environment variables in `.env` (especially `MAIL_API_KEY`).

5. Run the application:
   ```bash
   python app.py
   ```

## Compiling with PyInstaller

The `.env` file must be bundled alongside the executable so the application can read its configuration at runtime.

### Linux

```bash
pip install pyinstaller
pyinstaller --onefile --add-data ".env:." app.py
```

The compiled binary will be located at `dist/app`. Run it with:

```bash
./dist/app
```

### Windows

```bash
pip install pyinstaller
pyinstaller --onefile --add-data ".env;." app.py
```

> Note: On Windows the path separator inside `--add-data` is `;` instead of `:`.

The compiled executable will be located at `dist\app.exe`. Run it by double-clicking or from the terminal:

```bash
dist\app.exe
```

### Notes

- The `--onefile` flag bundles everything into a single executable.
- `--add-data ".env:."` (Linux) / `--add-data ".env;."` (Windows) ensures the `.env` file is included in the bundle and extracted to the same directory as the executable at runtime.
- Make sure the `.env` file contains the correct `MAIL_API_KEY` before compiling.