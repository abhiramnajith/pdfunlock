#!/usr/bin/env bash
# Download the official qpdf Windows build, verify its SHA-256, and copy
# qpdf.exe + its DLLs into src-tauri/binaries/. Windows resolves DLLs from the
# executable's own directory, so no rpath work is needed.
#
# Not run on the developer's macOS machine — invoked by the dispatcher only
# under Git Bash on Windows (CI). Requires curl, unzip, sha256sum (all present
# in the Git Bash environment on GitHub's windows-latest runner).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$REPO_ROOT/src-tauri/binaries"

# Pinned qpdf Windows release + its official SHA-256 (verified against the
# release's qpdf-<ver>.sha256 file). Bump both together when updating qpdf.
QPDF_VERSION="12.3.2"
QPDF_ZIP="qpdf-${QPDF_VERSION}-msvc64.zip"
QPDF_URL="https://github.com/qpdf/qpdf/releases/download/v${QPDF_VERSION}/${QPDF_ZIP}"
QPDF_SHA256="8941870a604e7c87ed24566b038d46c24ce76616254d2383c578f60c0677f202"

rm -rf "$DEST"; mkdir -p "$DEST"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$QPDF_URL" -o "$tmp/$QPDF_ZIP"
echo "${QPDF_SHA256}  ${tmp}/${QPDF_ZIP}" | sha256sum -c -
unzip -q "$tmp/$QPDF_ZIP" -d "$tmp/extracted"

# The msvc64 zip nests everything under qpdf-<ver>-msvc64/bin/. Locate qpdf.exe
# rather than hard-coding the path so a layout change doesn't silently break.
qpdf_exe="$(find "$tmp/extracted" -name qpdf.exe -print -quit)"
[ -n "$qpdf_exe" ] || { echo "error: qpdf.exe not found in $QPDF_ZIP" >&2; exit 1; }
bindir="$(dirname "$qpdf_exe")"

cp "$qpdf_exe" "$DEST/qpdf-x86_64-pc-windows-msvc.exe"
cp "$bindir"/*.dll "$DEST"/ 2>/dev/null || true

echo "Vendored Windows qpdf sidecar into $DEST:"
ls -la "$DEST"
