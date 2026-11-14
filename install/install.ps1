# install.ps1 - installs MochaEduCardReporter for the current user (Windows 10/11, 64-bit).
#
# Usage (PowerShell):
#   irm https://<install-host>/install.ps1 | iex
#
# Optional environment variables:
#   MOCHA_REPO=OWNER/REPO    read releases from this repository (overrides $Repo)
#   MOCHA_VERSION=vX.Y.Z     install this release tag instead of the latest one
#
# Works with Windows PowerShell 5.1 and PowerShell 7. Everything is inside
# Install-MochaEduCardReporter, which is called on the last line, so piping
# to iex does not leak variables into the user's session. Errors are reported
# and the function returns; the session is never closed.

function Install-MochaEduCardReporter {
    # >>> CONFIGURE: GitHub repository that publishes the releases, as OWNER/REPO.
    $Repo = "mochapulse/MochaEduCardReporter"
    # <<<

    $ErrorActionPreference = 'Stop'
    $productName = 'MochaEduCardReporter'
    $tempDir = $null

    try {
        if ($env:MOCHA_REPO) { $Repo = $env:MOCHA_REPO }
        if ([string]::IsNullOrEmpty($Repo) -or $Repo -eq 'OWNER/REPO') {
            throw "Este instalador todavia no esta configurado (falta el repositorio). Avisa a quien te lo envio."
        }
        if ($Repo -notmatch '^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$') {
            throw "El repositorio configurado no es valido ($Repo)."
        }

        # Requirements: 64-bit Windows 10 or 11.
        if (-not [Environment]::Is64BitOperatingSystem) {
            throw "Este programa necesita Windows de 64 bits."
        }
        if ([Environment]::OSVersion.Version.Major -lt 10) {
            throw "Este programa necesita Windows 10 o Windows 11."
        }

        # TLS 1.2 at least; TLS 1.3 too when the runtime knows it.
        $proto = [Net.SecurityProtocolType]::Tls12
        if ([Enum]::IsDefined([Net.SecurityProtocolType], 'Tls13')) {
            $proto = $proto -bor [Net.SecurityProtocolType]::Tls13
        }
        [Net.ServicePointManager]::SecurityProtocol = $proto

        $headers = @{ 'User-Agent' = 'mocha-installer'; 'Accept' = 'application/vnd.github+json' }

        # Find the release.
        $tag = $env:MOCHA_VERSION
        if ($tag) {
            if ($tag -notmatch '^[A-Za-z0-9._-]+$') {
                throw "MOCHA_VERSION contiene caracteres no validos."
            }
            Write-Host "Buscando la version $tag..."
            $api = "https://api.github.com/repos/$Repo/releases/tags/$tag"
        }
        else {
            Write-Host "Buscando la ultima version..."
            $api = "https://api.github.com/repos/$Repo/releases/latest"
        }
        try {
            $release = Invoke-RestMethod -Uri $api -Headers $headers -UseBasicParsing
        }
        catch {
            throw "No se pudo consultar la version en GitHub. Revisa tu conexion a internet y que la version exista."
        }
        $version = $release.tag_name
        if (-not $version) { $version = '(sin nombre)' }

        # Pick the installer and the checksum file.
        $setup = @($release.assets | Where-Object { $_.name -like '*-setup.exe' })
        if ($setup.Count -ne 1) {
            throw "La version $version no tiene un unico instalador para Windows. Avisa a quien te lo envio."
        }
        $sums = @($release.assets | Where-Object { $_.name -eq 'SHA256SUMS' })
        if ($sums.Count -ne 1) {
            throw "La version $version no incluye SHA256SUMS, asi que no se puede verificar la descarga. No se instalo nada."
        }
        foreach ($asset in @($setup[0], $sums[0])) {
            if ($asset.browser_download_url -notlike 'https://*') {
                throw "La direccion de descarga no es segura (no es HTTPS). No se instalo nada."
            }
        }

        $tempDir = Join-Path $env:TEMP ("mocha-install-" + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $tempDir | Out-Null
        $setupPath = Join-Path $tempDir $setup[0].name
        $sumsPath = Join-Path $tempDir 'SHA256SUMS'

        $dlHeaders = @{ 'User-Agent' = 'mocha-installer' }
        Write-Host "Descargando la lista de verificacion..."
        Invoke-WebRequest -Uri $sums[0].browser_download_url -OutFile $sumsPath -Headers $dlHeaders -UseBasicParsing

        Write-Host "Descargando $($setup[0].name) (puede tardar un poco)..."
        Invoke-WebRequest -Uri $setup[0].browser_download_url -OutFile $setupPath -Headers $dlHeaders -UseBasicParsing

        # Verify the SHA-256 before running anything.
        Write-Host "Verificando la descarga..."
        $wanted = $null
        foreach ($line in (Get-Content -LiteralPath $sumsPath)) {
            if ($line -match '^([0-9a-fA-F]{64})\s+\*?(.+?)\s*$' -and $Matches[2] -eq $setup[0].name) {
                $wanted = $Matches[1].ToLowerInvariant()
                break
            }
        }
        if (-not $wanted) {
            throw "SHA256SUMS no tiene una linea para $($setup[0].name). No se instalo nada."
        }
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $setupPath).Hash.ToLowerInvariant()
        if ($actual -ne $wanted) {
            Remove-Item -LiteralPath $setupPath -Force -ErrorAction SilentlyContinue
            throw "La verificacion fallo: el archivo descargado no coincide con el publicado. Se borro la descarga y no se instalo nada."
        }
        Write-Host "Verificacion correcta."

        # Silent per-user install (the NSIS installer is set to currentUser: no administrator rights).
        Write-Host "Instalando $productName $version..."
        $proc = Start-Process -FilePath $setupPath -ArgumentList '/S' -PassThru -Wait
        if ($proc.ExitCode -ne 0) {
            throw "El instalador termino con codigo $($proc.ExitCode). Revisa si el programa ya esta abierto y vuelve a intentarlo."
        }

        # Confirm the install through the uninstall registry entry.
        $entry = $null
        foreach ($key in (Get-ChildItem -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue)) {
            $props = Get-ItemProperty -Path $key.PSPath -ErrorAction SilentlyContinue
            if ($props -and $props.DisplayName -eq $productName) {
                $entry = $props
                break
            }
        }
        if (-not $entry) {
            throw "El instalador termino, pero no encuentro el programa instalado. Avisa a quien te lo envio."
        }

        # Start the app: the exe is the one the registry icon points to.
        $exe = $null
        if ($entry.DisplayIcon) {
            $candidate = ($entry.DisplayIcon -split ',')[0].Trim('"', ' ')
            if (Test-Path -LiteralPath $candidate) { $exe = $candidate }
        }
        if (-not $exe -and $entry.InstallLocation) {
            $found = Get-ChildItem -LiteralPath $entry.InstallLocation -Filter '*.exe' -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -notlike 'uninstall*' } | Select-Object -First 1
            if ($found) { $exe = $found.FullName }
        }

        if ($exe) {
            Start-Process -FilePath $exe
            Write-Host ""
            Write-Host "Listo: $productName $version esta instalado y se ha abierto."
        }
        else {
            Write-Host ""
            Write-Host "Listo: $productName $version esta instalado."
            Write-Host "Abrelo desde el menu Inicio (busca `"$productName`")."
        }
    }
    catch {
        Write-Host ""
        Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
        return
    }
    finally {
        if ($tempDir -and (Test-Path -LiteralPath $tempDir)) {
            Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Install-MochaEduCardReporter
