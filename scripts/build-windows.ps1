$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RootDir

$PythonBin = $env:PYTHON_BIN
if ([string]::IsNullOrWhiteSpace($PythonBin)) {
    $PythonBin = "py"
}

if (-not (Test-Path "venv")) {
    & $PythonBin -m venv venv
}

& .\venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
python -m pytest
python -m PyInstaller --clean --noconfirm app.spec

Write-Host "Build ready: $RootDir\dist\CoffeeEduMailer.exe"
