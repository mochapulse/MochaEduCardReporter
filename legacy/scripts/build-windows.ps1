$ErrorActionPreference = "Stop"

function Test-IsWindowsAdmin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-IsWindowsAdmin)) {
    $argList = @(
        "-ExecutionPolicy", "Bypass",
        "-NoProfile",
        "-File", "`"$PSCommandPath`""
    )

    try {
        Start-Process -FilePath "powershell.exe" -Verb RunAs -ArgumentList $argList -Wait
        exit $LASTEXITCODE
    }
    catch {
        throw "Administrator privileges are required for Windows deploy/build. Re-run PowerShell as Administrator and execute scripts/build-windows.ps1 again."
    }
}

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
python scripts/generate-icon.py
python -m PyInstaller --clean --noconfirm app.spec

Write-Host "Build ready: $RootDir\dist\MochaEduCardReporter.exe"
