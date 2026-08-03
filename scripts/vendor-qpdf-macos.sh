#!/usr/bin/env bash
# Vendor the Homebrew `qpdf` binary and its non-system dylibs into
# src-tauri/binaries/, rewriting every load command to @loader_path so the
# resulting set is fully self-contained (no /opt/homebrew or /usr/local
# references left) and can run from any directory as long as the vendored
# dylibs sit next to the qpdf binary.
#
# Verified dependency graph for qpdf 12.3.2 (aarch64-apple-darwin, Homebrew):
#   qpdf                -> @rpath/libqpdf.30.dylib   (LC_RPATH: @loader_path/../lib)
#                       -> /usr/lib/libc++.1.dylib      (system, left alone)
#                       -> /usr/lib/libSystem.B.dylib   (system, left alone)
#   libqpdf.30.dylib    -> /usr/lib/libz.1.dylib                          (system)
#                       -> .../jpeg-turbo/lib/libjpeg.8.dylib             (vendor)
#                       -> .../openssl@3/lib/libcrypto.3.dylib            (vendor)
#                       -> libc++ / libSystem                              (system)
#   libjpeg.8.dylib     -> libSystem only
#   libcrypto.3.dylib   -> libSystem only
#
# Note: the qpdf binary references libqpdf via @rpath (not a /opt or
# /usr/local path), so a naive grep for those prefixes in `otool -L` output
# misses it entirely. That edge is resolved explicitly below using the
# binary's LC_RPATH.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$REPO_ROOT/src-tauri/binaries"

# Derive the Rust target triple from the host arch so this works on both
# Apple Silicon and Intel runners (must match release.yml's rust_target).
case "$(uname -m)" in
  arm64|aarch64) TRIPLE="aarch64-apple-darwin" ;;
  x86_64)        TRIPLE="x86_64-apple-darwin" ;;
  *) echo "error: unsupported macOS arch $(uname -m)" >&2; exit 1 ;;
esac

# Check for qpdf BEFORE touching the destination, so a missing-qpdf run can't
# wipe an existing (still-valid) vendored set before failing.
if ! command -v qpdf >/dev/null 2>&1; then
  echo "error: qpdf not found on PATH — install it with: brew install qpdf" >&2
  exit 1
fi
SRC_QPDF="$(command -v qpdf)"

# Always start from a clean directory. Without this, re-running after e.g.
# `brew upgrade openssl@3` would silently keep the stale libcrypto.3.dylib
# (its basename carries no patch version, so a newer copy wouldn't overwrite
# a mismatched old one) — and a stale/mismatched vendored set is exactly the
# kind of bug that's invisible until runtime. This also makes the
# beforeBuildCommand regeneration in tauri.conf.json deterministic.
rm -rf "$DEST"
mkdir -p "$DEST"
# Resolve through symlinks (Homebrew's /opt/homebrew/bin/qpdf is a symlink
# into the Cellar) so we copy the real Mach-O binary.
SRC_QPDF="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$SRC_QPDF")"

DST_QPDF="$DEST/pdfunlock-qpdf-$TRIPLE"
cp "$SRC_QPDF" "$DST_QPDF"
chmod u+w "$DST_QPDF"

# Resolve an @rpath/... load command against a binary's LC_RPATH entries.
# The rpath is only meaningful relative to the binary's ORIGINAL location
# (e.g. the Homebrew Cellar), not wherever we've copied it to, so this takes
# the original (pre-copy) path explicitly.
resolve_rpath_dep() {
  local orig="$1" dep="$2"
  local suffix="${dep#@rpath/}"
  local bin_dir
  bin_dir="$(cd "$(dirname "$orig")" && pwd)"
  while IFS= read -r rpath; do
    local resolved="$rpath"
    resolved="${resolved/@loader_path/$bin_dir}"
    resolved="${resolved/@executable_path/$bin_dir}"
    # Collapse any ../ introduced by the substitution.
    resolved="$(cd "$resolved" 2>/dev/null && pwd || true)"
    if [ -n "$resolved" ] && [ -f "$resolved/$suffix" ]; then
      echo "$resolved/$suffix"
      return 0
    fi
  done < <(otool -l "$orig" | grep -A2 'cmd LC_RPATH' | sed -nE 's/^ *path (.*) \(offset.*\)$/\1/p')
  return 1
}

# Recursively vendor every non-system dependency of $1 (a file already
# inside $DEST, rewritten in place), given $2 = its original (pre-copy,
# symlink-resolved) source path, used only to resolve @rpath entries.
#
# $3 (top_level) is "1" only for the initial call on the main qpdf
# executable, "0" for every dylib-to-dylib edge found while recursing.
# The two cases need different rewritten load commands because the qpdf
# binary ends up in a *different directory* than the dylibs once bundled
# into a Tauri .app (externalBin -> Contents/MacOS/, resources ->
# Contents/Resources/binaries/), whereas dylib-to-dylib deps are always
# co-located, both in the raw src-tauri/binaries/ dir and inside the
# bundled app:
#   - top_level dep (qpdf -> libqpdf): rewritten to @rpath/<base>, and the
#     binary is given two LC_RPATH entries below ("@loader_path", for
#     running qpdf directly out of src-tauri/binaries/, and
#     "@loader_path/../Resources/binaries", for running it from inside the
#     bundled .app's Contents/MacOS/) so it resolves in both layouts.
#   - dylib-to-dylib dep: rewritten to plain @loader_path/<base>, which is
#     correct in both layouts since siblings stay siblings.
copy_deps() {
  local bin="$1" orig="$2" top_level="$3"
  local dep base resolved new_cmd
  while IFS= read -r dep; do
    [ -z "$dep" ] && continue
    case "$dep" in
      /usr/lib/*|/System/*)
        continue
        ;;
      @rpath/*)
        resolved="$(resolve_rpath_dep "$orig" "$dep")" || {
          echo "error: could not resolve $dep via rpath of $orig" >&2
          exit 1
        }
        ;;
      /opt/*|/usr/local/*)
        resolved="$dep"
        ;;
      *)
        # @loader_path/@executable_path self-references or anything already
        # vendored; nothing to do.
        continue
        ;;
    esac
    resolved="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$resolved")"
    base="$(basename "$resolved")"
    if [ ! -f "$DEST/$base" ]; then
      cp "$resolved" "$DEST/$base"
      chmod u+w "$DEST/$base"
      install_name_tool -id "@loader_path/$base" "$DEST/$base"
      copy_deps "$DEST/$base" "$resolved" 0
    fi
    if [ "$top_level" = "1" ]; then
      new_cmd="@rpath/$base"
    else
      new_cmd="@loader_path/$base"
    fi
    install_name_tool -change "$dep" "$new_cmd" "$bin"
  done < <(otool -L "$bin" | awk 'NR>1 {print $1}')
}

copy_deps "$DST_QPDF" "$SRC_QPDF" 1

# Every dependency is now referenced via an explicit @loader_path/<name>
# load command, so none of the vendored files need an LC_RPATH anymore.
# Strip whatever rpaths they carried over from their Homebrew originals
# (e.g. qpdf's "@loader_path/../lib" -> /opt/homebrew/lib, or libjpeg's
# absolute "/opt/homebrew/Cellar/.../lib") so no Homebrew path survives
# anywhere in the vendored set, including in LC_RPATH commands.
strip_rpaths() {
  local f="$1" rpath
  while IFS= read -r rpath; do
    [ -z "$rpath" ] && continue
    install_name_tool -delete_rpath "$rpath" "$f"
  done < <(otool -l "$f" | grep -A2 'cmd LC_RPATH' | sed -nE 's/^ *path (.*) \(offset.*\)$/\1/p')
}
for f in "$DEST"/*; do
  strip_rpaths "$f"
done

# Give the qpdf executable the two rpaths its @rpath/libqpdf... load command
# (added by copy_deps above) needs to resolve in each place it can run from:
# standalone next to its dylibs in src-tauri/binaries/, and staged inside a
# built .app where it lives in Contents/MacOS/ but the dylibs are Tauri
# "resources" under Contents/Resources/binaries/.
install_name_tool -add_rpath "@loader_path" "$DST_QPDF"
install_name_tool -add_rpath "@loader_path/../Resources/binaries" "$DST_QPDF"

# install_name_tool invalidates whatever code signature each file had
# (Homebrew ships ad-hoc/linker-signed Mach-O binaries on arm64). On Apple
# Silicon an unsigned or signature-mismatched binary is killed by the kernel
# at exec time (SIGKILL / exit 137), so every rewritten file must be
# re-signed ad-hoc before it's usable.
for f in "$DEST"/*; do
  codesign --sign - --force "$f"
done

echo "Vendored qpdf and dylibs into $DEST:"
ls -la "$DEST"
