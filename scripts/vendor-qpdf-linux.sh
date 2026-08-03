#!/usr/bin/env bash
# Vendor the system qpdf binary and its non-system .so deps into
# src-tauri/binaries/, setting RPATH=$ORIGIN so the binary finds its libs
# beside itself inside the bundle. Linux equivalent of vendor-qpdf-macos.sh.
#
# Not run on the developer's macOS machine — invoked by the dispatcher only on
# Linux (CI). Requires qpdf + patchelf installed (apt: qpdf patchelf).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$REPO_ROOT/src-tauri/binaries"

for tool in qpdf patchelf ldd; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "error: $tool not found (install with: apt-get install -y qpdf patchelf)" >&2
    exit 1
  }
done

# Rust target triple for naming the externalBin (must match release.yml matrix).
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64)  TRIPLE="x86_64-unknown-linux-gnu" ;;
  aarch64) TRIPLE="aarch64-unknown-linux-gnu" ;;
  *) echo "error: unsupported arch $ARCH" >&2; exit 1 ;;
esac

rm -rf "$DEST"; mkdir -p "$DEST"
SRC_QPDF="$(command -v qpdf)"
DST_QPDF="$DEST/pdfunlock-qpdf-$TRIPLE"
cp "$SRC_QPDF" "$DST_QPDF"
chmod u+w "$DST_QPDF"

# Copy every non-system shared library qpdf depends on. Exclude the core
# system set that is always present on a target machine (libc, libm, the
# dynamic loader, the vdso, and the C++/gcc runtimes that ship with every
# desktop distro), so we only bundle qpdf's real extras (libjpeg, libssl/crypto,
# etc.).
ldd "$SRC_QPDF" | awk '/=>/ {print $3} !/=>/ && /^\// {print $1}' | while read -r lib; do
  [ -z "$lib" ] && continue
  base="$(basename "$lib")"
  case "$base" in
    libc.so*|libm.so*|libdl.so*|libpthread.so*|librt.so*|ld-linux*|linux-vdso*|libgcc_s.so*|libstdc++.so*)
      continue ;;
  esac
  if [ -f "$lib" ] && [ ! -f "$DEST/$base" ]; then
    cp "$lib" "$DEST/$base"
    chmod u+w "$DEST/$base"
    patchelf --set-rpath '$ORIGIN' "$DEST/$base"
  fi
done

# The sidecar (externalBin) installs to <prefix>/bin/pdfunlock-qpdf, but Tauri
# puts the vendored .so files (resources) under <prefix>/lib/pdfunlock/binaries/
# — a different tree. So the binary needs two rpath entries:
#   $ORIGIN                          — running it standalone from src-tauri/binaries/
#                                      (and any layout that co-locates the libs)
#   $ORIGIN/../lib/pdfunlock/binaries — the installed deb/rpm/AppImage layout
# The .so files themselves stay $ORIGIN since they're always co-located.
patchelf --set-rpath '$ORIGIN:$ORIGIN/../lib/pdfunlock/binaries' "$DST_QPDF"

echo "Vendored Linux qpdf sidecar into $DEST:"
ls -la "$DEST"
"$DST_QPDF" --version
