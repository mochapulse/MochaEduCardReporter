# uninstall.ps1 - removes MochaEduCardReporter for the current user (Windows 10/11).
#
# Usage (PowerShell):
#   powershell -ExecutionPolicy Bypass -File uninstall.ps1
#   or: irm https://<install-host>/uninstall.ps1 | iex
#
# Runs the uninstaller that the per-user install registered, silently (/S).
# Your documents and generated PDFs are NOT touched.
# Works with Windows PowerShell 5.1 and PowerShell 7. Errors are reported and
# the function returns; the session is never closed.

function Uninstall-MochaEduCardReporter {
    $ErrorActionPreference = 'Stop'
    # Must match the DisplayName written by the installer.
    $productName = 'MochaEduCardReporter'

    try {
        $entry = $null
        foreach ($key in (Get-ChildItem -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue)) {
            $props = Get-ItemProperty -Path $key.PSPath -ErrorAction SilentlyContinue
            if ($props -and $props.DisplayName -eq $productName) {
                $entry = $props
                break
            }
        }
        if (-not $entry) {
            Write-Host "No encuentro $productName instalado en este usuario. No se ha cambiado nada."
            return
        }
        if (-not $entry.UninstallString) {
            throw "No encuentro el desinstalador de $productName. Avisa a quien te lo envio."
        }

        # UninstallString is usually: "C:\...\uninstall.exe" (possibly with arguments).
        $cmd = [string]$entry.UninstallString
        if ($cmd -match '^\s*"([^"]+)"\s*(.*)$') {
            $uninstExe = $Matches[1]
        }
        else {
            $uninstExe = ($cmd -split '\s+')[0]
        }
        if (-not (Test-Path -LiteralPath $uninstExe)) {
            throw "No encuentro el archivo del desinstalador ($uninstExe). Avisa a quien te lo envio."
        }

        Write-Host "Desinstalando $productName..."
        $proc = Start-Process -FilePath $uninstExe -ArgumentList '/S' -PassThru -Wait
        if ($proc.ExitCode -ne 0) {
            throw "El desinstalador termino con codigo $($proc.ExitCode). Cierra el programa si esta abierto y vuelve a intentarlo."
        }

        Write-Host ""
        Write-Host "$productName se ha desinstalado."
        Write-Host "Tus documentos y los PDF generados NO se han tocado."
    }
    catch {
        Write-Host ""
        Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
        return
    }
}

Uninstall-MochaEduCardReporter
