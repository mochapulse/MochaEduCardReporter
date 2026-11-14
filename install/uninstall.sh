#!/bin/sh
# uninstall.sh - removes MochaEduCardReporter from Linux.
#
# Usage:
#   sh uninstall.sh
#
# Detects how the app was installed (deb package, rpm package, or AppImage
# files) and removes that install, plus the menu entry and command link.
# Documents, generated PDFs and the app's data folders are never touched.
# POSIX sh only.

# >>> CONFIGURE: package name used by the .deb and .rpm (check with
# `dpkg-deb -f <file>.deb Package` on a real build). Must be lower case.
PACKAGE="mochaeducardreporter"
# <<<

APP_NAME="MochaEduCardReporter"
BIN_NAME="mocha-edu-card-reporter"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
DATA_DIR="$DATA_HOME/$APP_NAME"
APPS_DIR="$DATA_HOME/applications"
BIN_DIR="$HOME/.local/bin"
APPIMAGE="$DATA_DIR/$APP_NAME.AppImage"
DESKTOP="$APPS_DIR/$BIN_NAME.desktop"
LINK="$BIN_DIR/$BIN_NAME"

say() { printf '%s\n' "$*"; }
warn() { printf 'Aviso: %s\n' "$*" >&2; }
die() { printf '\nError: %s\n' "$*" >&2; exit 1; }

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

deb_installed() {
    command -v dpkg-query >/dev/null 2>&1 || return 1
    dpkg-query -W -f='${Status}' "$PACKAGE" 2>/dev/null | grep -q '^install ok installed'
}

rpm_installed() {
    command -v rpm >/dev/null 2>&1 || return 1
    rpm -q "$PACKAGE" >/dev/null 2>&1
}

appimage_installed() {
    [ -e "$APPIMAGE" ] || [ -L "$LINK" ] || [ -e "$DESKTOP" ]
}

remove_deb() {
    say "Quitando el paquete .deb con apt."
    run_as_root apt-get remove -y "$PACKAGE" \
        || die "apt no pudo quitar el paquete. Revisa el mensaje de arriba."
}

remove_rpm() {
    say "Quitando el paquete .rpm."
    if command -v dnf >/dev/null 2>&1; then
        run_as_root dnf remove -y "$PACKAGE" || die "dnf no pudo quitar el paquete."
    elif command -v zypper >/dev/null 2>&1; then
        run_as_root zypper --non-interactive remove "$PACKAGE" || die "zypper no pudo quitar el paquete."
    else
        run_as_root rpm -e "$PACKAGE" || die "rpm no pudo quitar el paquete."
    fi
}

remove_appimage() {
    say "Quitando la versión AppImage."
    # Remove the command link only when it points to our AppImage.
    if [ -L "$LINK" ] && [ "$(readlink "$LINK")" = "$APPIMAGE" ]; then
        rm -f -- "$LINK"
    fi
    rm -f -- "$APPIMAGE" "$DESKTOP"
    # Remove the folder only if it is now empty (never delete other files).
    rmdir -- "$DATA_DIR" 2>/dev/null || true
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
    fi
}

main() {
    _found=0
    if deb_installed; then
        _found=1
        say "Encontrado: paquete .deb ($PACKAGE)."
        remove_deb
    fi
    if rpm_installed; then
        _found=1
        say "Encontrado: paquete .rpm ($PACKAGE)."
        remove_rpm
    fi
    if appimage_installed; then
        _found=1
        say "Encontrada: versión AppImage."
        remove_appimage
    fi

    if [ "$_found" -eq 0 ]; then
        say "No encontré $APP_NAME instalado en este equipo. No se ha cambiado nada."
        return 0
    fi

    say ""
    say "$APP_NAME se ha desinstalado."
    say "Tus documentos y los PDF generados NO se han tocado."
}

main "$@"
