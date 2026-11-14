#!/bin/sh
# install.sh - installs MochaEduCardReporter on Linux (x86_64).
#
# Usage:
#   curl -fsSL https://<install-host>/install.sh | sh
#
# Optional environment variables:
#   MOCHA_REPO=OWNER/REPO    read releases from this repository (overrides REPO)
#   MOCHA_VERSION=vX.Y.Z     install this release tag instead of the latest one
#   MOCHA_FORCE_APPIMAGE=1   always use the AppImage (needs no root)
#
# POSIX sh only. Everything lives in functions and main() is called on the
# last line, so a truncated download cannot run a half-written script.
# Set MOCHA_INSTALL_LIB=1 to load the functions without running main (tests).

# >>> CONFIGURE: GitHub repository that publishes the releases, as OWNER/REPO.
REPO="mochapulse/MochaEduCardReporter"
# <<<
REPO="${MOCHA_REPO:-$REPO}"

APP_NAME="MochaEduCardReporter"
APP_COMMENT="Boletines de notas en PDF a partir de una hoja de cálculo"
BIN_NAME="mocha-edu-card-reporter"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
DATA_DIR="$DATA_HOME/$APP_NAME"
APPS_DIR="$DATA_HOME/applications"
BIN_DIR="$HOME/.local/bin"

TMP_DIR=""
TAG=""
METHOD=""
RPM_TOOL=""
ASSET_NAME=""
ASSET_URL=""

say() { printf '%s\n' "$*"; }
warn() { printf 'Aviso: %s\n' "$*" >&2; }
die() { printf '\nError: %s\n' "$*" >&2; exit 1; }

cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf -- "$TMP_DIR"
    fi
}

# Prints the value of KEY from /etc/os-release (empty if unknown).
os_value() {
    sed -n "s/^$1=//p" /etc/os-release 2>/dev/null | head -n 1 | tr -d '"'
}

check_platform() {
    [ "$(uname -s)" = "Linux" ] || die "Este instalador es solo para Linux."
    case $(uname -m) in
        x86_64|amd64) ;;
        *) die "Este instalador necesita un equipo de 64 bits (x86_64). Tu equipo es $(uname -m)." ;;
    esac
}

check_repo() {
    case $REPO in
        ""|OWNER/REPO)
            die "Este instalador todavía no está configurado (falta el repositorio). Avisa a quien te lo envió." ;;
    esac
    case $REPO in
        */*) ;;
        *) die "El repositorio configurado no es válido ($REPO)." ;;
    esac
    case $REPO in
        *[!A-Za-z0-9._/-]*) die "El repositorio configurado no es válido ($REPO)." ;;
    esac
}

check_tools() {
    _missing=""
    for _cmd in sha256sum awk sed grep tr mktemp uname; do
        command -v "$_cmd" >/dev/null 2>&1 || _missing="$_missing $_cmd"
    done
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        _missing="$_missing curl(o wget)"
    fi
    [ -z "$_missing" ] || die "Faltan programas necesarios:$_missing. Instálalos e inténtalo de nuevo."
}

# Downloads URL to DEST. Only https:// URLs are accepted.
download() { # $1 = URL, $2 = destination
    case $1 in
        https://*) ;;
        *) return 1 ;;
    esac
    if command -v curl >/dev/null 2>&1; then
        curl --fail --location --proto '=https' --proto-redir '=https' --tlsv1.2 \
            --retry 2 --progress-bar -o "$2" "$1"
    else
        wget -O "$2" "$1"
    fi
}

# Fetches a GitHub API URL (JSON) to DEST.
api_get() { # $1 = URL, $2 = destination
    case $1 in
        https://*) ;;
        *) return 1 ;;
    esac
    if command -v curl >/dev/null 2>&1; then
        curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
            --tlsv1.2 -H 'Accept: application/vnd.github+json' -H 'User-Agent: mocha-installer' \
            -o "$2" "$1"
    else
        wget -q --header='Accept: application/vnd.github+json' --user-agent='mocha-installer' \
            -O "$2" "$1"
    fi
}

# Reads the release JSON and writes the download URLs (one per line) to urls.txt.
fetch_release() {
    if [ -n "${MOCHA_VERSION:-}" ]; then
        case $MOCHA_VERSION in
            *[!A-Za-z0-9._-]*) die "MOCHA_VERSION contiene caracteres no válidos." ;;
        esac
        _api="https://api.github.com/repos/$REPO/releases/tags/$MOCHA_VERSION"
        say "Buscando la versión $MOCHA_VERSION..."
    else
        _api="https://api.github.com/repos/$REPO/releases/latest"
        say "Buscando la última versión..."
    fi
    api_get "$_api" "$TMP_DIR/release.json" \
        || die "No se pudo consultar la versión en GitHub. Revisa tu conexión a internet y que la versión exista."

    TAG=$(grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' "$TMP_DIR/release.json" \
        | head -n 1 | sed 's/.*"\([^"]*\)"$/\1/')
    [ -n "$TAG" ] || TAG="(sin nombre)"

    grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*"' "$TMP_DIR/release.json" \
        | sed 's/.*"\([^"]*\)"$/\1/' > "$TMP_DIR/urls.txt"
    [ -s "$TMP_DIR/urls.txt" ] || die "La versión $TAG no tiene archivos para descargar."
}

# Prints every URL in FILE whose file name matches the glob PATTERN.
find_assets() { # $1 = glob on the file name, $2 = URL list file
    while IFS= read -r _url || [ -n "$_url" ]; do
        _name=${_url##*/}
        # shellcheck disable=SC2254
        case $_name in
            $1) printf '%s\n' "$_url" ;;
        esac
    done < "$2"
}

# Prints the single URL matching PATTERN. Prints nothing and fails when there
# is no match or more than one match.
pick_asset() { # $1 = glob on the file name, $2 = URL list file
    _list=$(find_assets "$1" "$2")
    _count=$(printf '%s' "$_list" | grep -c '')
    [ "$_count" -eq 1 ] || return 1
    printf '%s\n' "$_list"
}

# Chooses deb, rpm or appimage from the distribution and the tools available.
choose_method() {
    METHOD="appimage"
    if [ "${MOCHA_FORCE_APPIMAGE:-0}" = "1" ]; then
        say "Se usará la versión AppImage (MOCHA_FORCE_APPIMAGE=1)."
        return 0
    fi
    _family=" $(os_value ID) $(os_value ID_LIKE) "
    case $_family in
        *debian*|*ubuntu*)
            if command -v apt-get >/dev/null 2>&1; then METHOD="deb"; fi ;;
        *fedora*|*rhel*|*suse*)
            if command -v dnf >/dev/null 2>&1; then
                METHOD="rpm"; RPM_TOOL="dnf"
            elif command -v zypper >/dev/null 2>&1; then
                METHOD="rpm"; RPM_TOOL="zypper"
            fi ;;
    esac
    if [ "$METHOD" != "appimage" ] && [ "$(id -u)" -ne 0 ] && ! command -v sudo >/dev/null 2>&1; then
        warn "no hay sudo, así que no se puede instalar un paquete del sistema. Se usará la versión AppImage, que no necesita permisos de administrador."
        METHOD="appimage"
    fi
}

# Sets ASSET_URL and ASSET_NAME for METHOD, falling back to AppImage.
select_asset() {
    case $METHOD in
        deb) _pattern='*_amd64.deb' ;;
        rpm) _pattern='*.x86_64.rpm' ;;
        *) _pattern='*_amd64.AppImage' ;;
    esac
    ASSET_URL=$(pick_asset "$_pattern" "$TMP_DIR/urls.txt") || ASSET_URL=""
    if [ -z "$ASSET_URL" ] && [ "$METHOD" != "appimage" ]; then
        warn "La versión $TAG no tiene paquete para este sistema; se usará la versión AppImage."
        METHOD="appimage"
        ASSET_URL=$(pick_asset '*_amd64.AppImage' "$TMP_DIR/urls.txt") || ASSET_URL=""
    fi
    [ -n "$ASSET_URL" ] || die "La versión $TAG no tiene el archivo de instalación para este equipo."
    ASSET_NAME=${ASSET_URL##*/}
}

# Prints the SHA-256 listed for NAME in SUMS (lowercase), or nothing.
expected_sha256() { # $1 = SHA256SUMS file, $2 = file name
    awk -v name="$2" '{ f = $2; sub(/^\*/, "", f); if (f == name) { print $1; exit } }' "$1" \
        | tr 'A-F' 'a-f'
}

# Returns 0 if FILE matches its entry in SUMS; 1 if the entry is missing or
# malformed; 2 if the hash differs.
verify_sha256() { # $1 = file, $2 = SHA256SUMS file, $3 = file name as listed
    _want=$(expected_sha256 "$2" "$3")
    printf '%s' "$_want" | grep -Eq '^[0-9a-f]{64}$' || return 1
    _got=$(sha256sum "$1" | awk '{ print $1 }')
    [ "$_got" = "$_want" ] || return 2
    return 0
}

# Downloads SHA256SUMS and the chosen file, then verifies the hash.
fetch_verified_asset() {
    _sums_url=$(pick_asset 'SHA256SUMS' "$TMP_DIR/urls.txt") \
        || die "La versión $TAG no incluye SHA256SUMS, así que no se puede verificar la descarga. No se instaló nada."
    say "Descargando la lista de verificación..."
    download "$_sums_url" "$TMP_DIR/SHA256SUMS" \
        || die "No se pudo descargar SHA256SUMS. Revisa tu conexión a internet. No se instaló nada."

    say "Descargando $ASSET_NAME (puede tardar un poco)..."
    download "$ASSET_URL" "$TMP_DIR/$ASSET_NAME" \
        || die "No se pudo descargar $ASSET_NAME. Revisa tu conexión a internet. No se instaló nada."

    say "Verificando la descarga..."
    _rc=0
    verify_sha256 "$TMP_DIR/$ASSET_NAME" "$TMP_DIR/SHA256SUMS" "$ASSET_NAME" || _rc=$?
    case $_rc in
        0) say "Verificación correcta." ;;
        1) die "SHA256SUMS no tiene una línea válida para $ASSET_NAME. No se instaló nada." ;;
        *)
            rm -f -- "$TMP_DIR/$ASSET_NAME"
            die "La verificación falló: el archivo descargado no coincide con el publicado. Se borró la descarga y no se instaló nada." ;;
    esac
}

# Runs a command as root. Shows the command, and uses sudo when not root.
run_as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        say "  -> $*"
        "$@" </dev/null
    else
        say "  -> sudo $*"
        say "     (Puede pedirte tu contraseña de administrador.)"
        sudo "$@" </dev/null
    fi
}

install_deb() {
    say "Instalando el paquete .deb con apt (se resuelven las dependencias)."
    run_as_root apt-get install -y "$TMP_DIR/$ASSET_NAME" \
        || die "apt no pudo instalar el paquete. Revisa el mensaje de arriba."
}

install_rpm() {
    say "Instalando el paquete .rpm con $RPM_TOOL."
    case $RPM_TOOL in
        dnf) run_as_root dnf install -y "$TMP_DIR/$ASSET_NAME" ;;
        zypper) run_as_root zypper --non-interactive install --allow-unsigned-rpm "$TMP_DIR/$ASSET_NAME" ;;
    esac || die "$RPM_TOOL no pudo instalar el paquete. Revisa el mensaje de arriba."
}

install_appimage() {
    _target="$DATA_DIR/$APP_NAME.AppImage"
    _link="$BIN_DIR/$BIN_NAME"
    _desktop="$APPS_DIR/$BIN_NAME.desktop"

    mkdir -p "$DATA_DIR" "$BIN_DIR" "$APPS_DIR" \
        || die "No se pudieron crear las carpetas de instalación en tu usuario."
    cp -f -- "$TMP_DIR/$ASSET_NAME" "$_target.new" \
        || die "No se pudo copiar el programa a $DATA_DIR."
    chmod 755 "$_target.new"
    mv -f -- "$_target.new" "$_target" || die "No se pudo instalar el programa en $DATA_DIR."

    ln -sfn "$_target" "$_link" || die "No se pudo crear el comando en $BIN_DIR."

    # Desktop Entry: Exec needs quotes, and \ " ` $ and % need escaping.
    _exec=$(printf '%s' "$_target" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' \
        -e 's/`/\\`/g' -e 's/\$/\\$/g' -e 's/%/%%/g')
    # No icon is extracted from the AppImage (that would run code from it),
    # so a generic freedesktop icon name is used.
    {
        printf '%s\n' '[Desktop Entry]' 'Type=Application' "Name=$APP_NAME" \
            "Comment=$APP_COMMENT" "Exec=\"$_exec\"" 'Icon=x-office-document' \
            'Terminal=false' 'Categories=Education;Office;' 'StartupNotify=true'
    } > "$_desktop" || die "No se pudo crear el acceso en el menú de aplicaciones."

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
    fi

    # An AppImage mounts itself with FUSE 2; without it the program will not start.
    if command -v ldconfig >/dev/null 2>&1 \
        && ! ldconfig -p 2>/dev/null | grep -q 'libfuse\.so\.2'; then
        warn "el AppImage necesita el componente «fuse2», que no está instalado. Instálalo antes de abrir el programa (Arch: sudo pacman -S fuse2; openSUSE: sudo zypper install fuse2) o instala el paquete de tu distribución con MOCHA_FORCE_APPIMAGE=0."
    fi

    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *) warn "la carpeta $BIN_DIR no está en tu PATH. El menú de aplicaciones funciona igual; el comando '$BIN_NAME' no. Añade esa carpeta a tu PATH si lo necesitas." ;;
    esac
}

report_success() {
    say ""
    say "Listo: $APP_NAME $TAG está instalado."
    say "Ábrelo desde el menú de aplicaciones (busca \"$APP_NAME\")."
}

main() {
    trap cleanup EXIT
    trap 'cleanup; exit 130' INT
    trap 'cleanup; exit 143' TERM

    check_platform
    check_repo
    check_tools

    TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/mocha-install.XXXXXX") \
        || die "No se pudo crear una carpeta temporal."

    fetch_release
    choose_method
    select_asset
    say "Versión: $TAG. Método de instalación: $METHOD."
    fetch_verified_asset

    case $METHOD in
        deb) install_deb ;;
        rpm) install_rpm ;;
        appimage) install_appimage ;;
    esac

    report_success
}

[ "${MOCHA_INSTALL_LIB:-0}" = "1" ] || main "$@"
