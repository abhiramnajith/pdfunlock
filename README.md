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

### Requirements

- **macOS** on Apple Silicon (arm64).

There is no prebuilt release yet, so install by building from source (below). Building produces a normal macOS `.app` you can move to `/Applications`.

### Build from source

**1. Install the toolchain**

```bash
# Rust (if you don't have it): https://rustup.rs
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js 20+ and qpdf + python3 (used only when building)
brew install node qpdf python3

# Tauri CLI
cargo install tauri-cli --version "^2.0" --locked
```

**2. Clone and install dependencies**

```bash
git clone https://github.com/abhiramnajith/pdfunlock.git
cd pdfunlock
npm install
```

**3. Build the app**

```bash
npm run tauri build
```

The finished app is at:

```
src-tauri/target/release/bundle/macos/pdfunlock.app
```

Drag it to your `/Applications` folder. The `.app` bundles its own copy of `qpdf`, so it runs on any Mac without needing `qpdf` installed.

> Because this build is not code-signed, the first time you open it macOS may warn that it's from an unidentified developer. Right-click the app → **Open** → **Open**, or allow it under **System Settings → Privacy & Security**.

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

### Building a release bundle

The release `.app` ships qpdf as a self-contained sidecar (no system/brew qpdf required at runtime). To build it, the build machine needs:

- `brew install qpdf`
- `python3` available on PATH

`npm run tauri build` runs `scripts/vendor-qpdf.sh` automatically (wired in as `beforeBuildCommand`), which vendors qpdf and its non-system dylibs into `src-tauri/binaries/` (gitignored — regenerated on every build, not checked into the repo).

## License

MIT
