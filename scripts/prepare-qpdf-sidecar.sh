#!/usr/bin/env bash
# Vendors the qpdf sidecar for the current OS into src-tauri/binaries/.
# Wired as Tauri's beforeBuildCommand so local and CI builds both work.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
case "$(uname -s)" in
  Darwin)              exec bash "$DIR/vendor-qpdf-macos.sh" ;;
  Linux)               exec bash "$DIR/vendor-qpdf-linux.sh" ;;
  MINGW*|MSYS*|CYGWIN*) exec bash "$DIR/vendor-qpdf-windows.sh" ;;
  *) echo "error: unsupported OS $(uname -s) for qpdf vendoring" >&2; exit 1 ;;
esac
