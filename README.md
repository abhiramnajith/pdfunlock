# PDFUnlock

A small, private macOS desktop app that **removes password protection from PDFs you know the password to**. Drop in one or more locked PDFs, enter the password, and get decrypted copies saved right next to the originals.

Everything happens **entirely on your Mac** — no file ever leaves the device, and there is no network access.

> PDFUnlock only *removes a password you already know*. It does not recover, guess, or brute-force unknown passwords.

## Features

- **Batch unlocking** — drop several PDFs that share a password and unlock them all at once, with per-file status.
- **Drag-and-drop** or a native file picker.
- **Never destroys originals** — decrypted copies are saved as `name-unlocked.pdf` next to the source (with ` (2)`, ` (3)`… if that name already exists).
- **Handles every common scheme** — RC4 (40/128-bit), AES-128, and AES-256, powered by [qpdf](https://qpdf.sourceforge.io/).
- **Skips files that aren't encrypted** (with a note) and keeps going if one file has the wrong password.
- **Polished light/dark UI** that follows your system appearance.
- **Reveal in Finder** for each unlocked file.

## How it works

PDFUnlock is a [Tauri v2](https://tauri.app/) app — a Rust backend with a React + TypeScript frontend. The Rust side shells out to `qpdf` to detect encryption and decrypt, passing the password over stdin (never on the command line). The password never touches disk and is never logged.

---

## Install

Download the latest build for your OS from the
[**Releases**](https://github.com/abhiramnajith/pdfunlock/releases) page. qpdf is
bundled inside every download — there's nothing else to install.

| OS | Download | First-run note |
|----|----------|----------------|
| macOS (Apple Silicon) | `pdfunlock_*_aarch64.dmg` | Unsigned: right-click the app → **Open** → **Open** |
| macOS (Intel) | `pdfunlock_*_x64.dmg` | Same right-click → **Open** |
| Windows | `pdfunlock_*_x64-setup.exe` or `.msi` | SmartScreen: **More info → Run anyway** |
| Linux (x86-64 / arm64) | `.AppImage`, `.deb`, or `.rpm` | AppImage: `chmod +x pdfunlock_*.AppImage`, then run it |

Builds are **not code-signed**, hence the one-time OS warning above.

### Verify your download (recommended)

Every release includes a `SHA256SUMS` file and signed build-provenance
attestations:

```bash
# Checksum — compare against the SHA256SUMS file on the release
shasum -a 256 <downloaded-file>          # macOS
sha256sum <downloaded-file>              # Linux

# Provenance — proves the file came from this repo's Release workflow
# (needs the GitHub CLI: https://cli.github.com)
gh attestation verify <downloaded-file> --repo abhiramnajith/pdfunlock
```

### Build from source

Prefer building yourself? You'll need Rust, Node 20+, and qpdf.

```bash
# Toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust
brew install node qpdf python3                                    # macOS deps
cargo install tauri-cli --version "^2.0" --locked

# Build
git clone https://github.com/abhiramnajith/pdfunlock.git
cd pdfunlock
npm install
npm run tauri build
```

`npm run tauri build` runs `scripts/prepare-qpdf-sidecar.sh` automatically to
vendor qpdf into the bundle. The finished installer lands under
`src-tauri/target/release/bundle/`. On Linux the build also needs
`patchelf` and the usual Tauri/WebKitGTK dev packages; on Windows qpdf is
downloaded automatically by the vendor script.

## Usage

1. Open PDFUnlock.
2. Drag one or more password-protected PDFs onto the drop zone (or click **Browse**).
3. Type the password shared by those files.
4. Click **Unlock all**.
5. Each row shows its result; click **Reveal in Finder** on an unlocked file to jump to `…-unlocked.pdf`.

---

## Development

```bash
# Run the app with hot-reload
npm run tauri dev

# Frontend unit tests
npm test

# Rust tests (requires qpdf installed: `brew install qpdf`)
cd src-tauri && cargo test
```

### Recommended IDE setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### Releases & the qpdf sidecar

Every build bundles qpdf as a self-contained sidecar, so end users need no
system qpdf. `npm run tauri build` runs `scripts/prepare-qpdf-sidecar.sh`
(wired as `beforeBuildCommand`), which dispatches by OS:

- **macOS** — `vendor-qpdf-macos.sh`: copies the dylibs and rewrites load paths (`install_name_tool`). Needs `brew install qpdf` + `python3`.
- **Linux** — `vendor-qpdf-linux.sh`: copies the `.so`s and sets `RPATH=$ORIGIN` (`patchelf`). Needs `qpdf` + `patchelf`.
- **Windows** — `vendor-qpdf-windows.sh`: downloads the official qpdf build, verifies its SHA-256, and copies `qpdf.exe` + DLLs.

`src-tauri/binaries/` is gitignored and regenerated on every build.

Releases are produced by **GitHub Actions** (`.github/workflows/release.yml`)
on any `v*` tag: it builds all five targets, attaches the installers plus a
`SHA256SUMS` file, and generates build-provenance attestations. `ci.yml` runs
tests, audits, and CodeQL on every push/PR.

## License

**MIT + [Commons Clause](https://commonsclause.com/)** — source-available.

You may use, copy, modify, and distribute PDFUnlock freely, including within a
business. You may **not "Sell" it** — i.e., provide it (or a product/service
whose value derives substantially from it, including paid hosting or support as
the product) to third parties for a fee — without permission. Copyright remains
with the author.

For a commercial/resale license, open an issue to get in touch.

This is source-available, not OSI "open source." Not legal advice.
