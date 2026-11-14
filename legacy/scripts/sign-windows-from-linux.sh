#!/usr/bin/env bash
set -euo pipefail

TARGET_EXE="${1:-dist/MochaEduCardReporter.exe}"
SIGNED_EXE="${2:-${TARGET_EXE%.exe}-signed.exe}"
TIMESTAMP_URL="${WINDOWS_SIGN_TIMESTAMP_URL:-http://timestamp.digicert.com}"

if ! command -v osslsigncode >/dev/null 2>&1; then
  echo "osslsigncode is required. On Debian/Ubuntu: sudo apt install osslsigncode" >&2
  exit 1
fi

if [[ -z "${WINDOWS_SIGN_CERT:-}" ]]; then
  echo "Set WINDOWS_SIGN_CERT to the path of your .pfx/.p12 code-signing certificate." >&2
  exit 1
fi

if [[ -z "${WINDOWS_SIGN_PASSWORD:-}" ]]; then
  echo "Set WINDOWS_SIGN_PASSWORD to the certificate password." >&2
  exit 1
fi

osslsigncode sign \
  -pkcs12 "$WINDOWS_SIGN_CERT" \
  -pass "$WINDOWS_SIGN_PASSWORD" \
  -n "MochaEduCardReporter" \
  -i "https://github.com/dramirezbe/MochaEduCardReporter" \
  -t "$TIMESTAMP_URL" \
  -in "$TARGET_EXE" \
  -out "$SIGNED_EXE"

osslsigncode verify -in "$SIGNED_EXE"

echo "Signed executable ready: $SIGNED_EXE"
