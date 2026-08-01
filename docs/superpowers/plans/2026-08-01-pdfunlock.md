# PDFUnlock Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a macOS desktop app that removes password protection from PDFs the user has the password to, in batch, saving decrypted copies alongside the originals.

**Architecture:** Tauri v2 app — React + Vite + TypeScript frontend, Rust backend. The Rust backend exposes a single `unlock_pdf` command that shells out to a bundled `qpdf` binary to detect encryption and decrypt. The frontend calls the command once per file and updates that file's row independently, so one failure never halts the batch.

**Tech Stack:** Tauri v2, Rust, React 18 + TypeScript + Vite, vitest (frontend tests), qpdf (PDF engine).

## Global Constraints

- **Platform:** macOS-first, Apple Silicon (`aarch64-apple-darwin`) is the primary target.
- **Tauri version:** v2 (not v1).
- **Node is not on PATH.** Every command that needs node/npm must be prefixed with this export (Zed's bundled node v24.11.0):
  ```bash
  export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
  ```
- **qpdf** is the PDF engine. Install for dev with `brew install qpdf`. Bundled as a Tauri sidecar for distribution (Task 9).
- **Never modify or overwrite the original file.** Output goes to a sibling file named `<stem>-unlocked.<ext>`, with ` (2)`, ` (3)`, … on collision.
- **Password is never passed on the command line / argv.** It is written to qpdf's stdin via `--password-file=-`.
- **No network access.** All processing is local.
- **Batch continues on per-file error** — a failure on one file must never abort processing of the others.
- Test password for all fixtures is the literal string `secret`.

---

## File Structure

```
pdfunlock/
├── src/                          # React frontend
│   ├── main.tsx                  # React entry (from scaffold)
│   ├── App.tsx                   # Root UI: drop zone + list + password + action
│   ├── types.ts                  # UnlockOutcome / UnlockError TS mirrors
│   ├── unlock.ts                 # invoke() wrapper + batch orchestration
│   ├── unlock.test.ts            # vitest tests for orchestration
│   └── styles.css                # app styling (theme, layout)
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json           # window, bundle, externalBin (sidecar)
│   ├── binaries/                 # bundled qpdf + dylibs (Task 9)
│   ├── src/
│   │   ├── main.rs               # entry (from scaffold)
│   │   ├── lib.rs                # tauri builder + `unlock_pdf` command
│   │   └── pdf/
│   │       ├── mod.rs            # UnlockOutcome, UnlockError, unlock_pdf_impl
│   │       ├── naming.rs         # unique_output_path
│   │       └── qpdf.rs           # resolve_qpdf, is_encrypted, decrypt, errors
│   └── tests/
│       ├── common/mod.rs         # fixture generation helper
│       ├── naming.rs             # (unit tests live in naming.rs; see Task 2)
│       └── unlock.rs             # integration tests for the pipeline
└── docs/superpowers/…            # spec + this plan (already committed)
```

---

## Task 1: Toolchain, scaffold, and a running blank app

**Files:**
- Create: entire Tauri v2 scaffold at repo root (`src/`, `src-tauri/`, `package.json`, `vite.config.ts`, etc.)

**Interfaces:**
- Produces: a runnable Tauri dev app; the `npm run tauri dev` workflow used by all later tasks.

- [ ] **Step 1: Install qpdf (dev engine)**

```bash
brew install qpdf
qpdf --version   # expect qpdf version 11.x or 12.x
```

- [ ] **Step 2: Install the Tauri CLI**

```bash
cargo install tauri-cli --version "^2.0" --locked
cargo tauri --version   # expect tauri-cli 2.x
```

- [ ] **Step 3: Scaffold a Tauri v2 React-TS app into a temp dir**

The repo root already contains `docs/`, `.git`, `.gitignore`, so scaffold elsewhere and copy in.

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
cd /tmp
npm create tauri-app@latest pdfunlock-scaffold -- --template react-ts --manager npm --identifier com.pdfunlock.app --yes
```

- [ ] **Step 4: Copy scaffold into the repo (preserving docs/ and .git)**

```bash
cd /Users/abhiramnajith/Documents/claude-projects/pdfunlock
rsync -a --exclude='.git' --exclude='docs' /tmp/pdfunlock-scaffold/ ./
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
npm install
```

- [ ] **Step 5: Run the app to verify it launches**

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
npm run tauri dev
```
Expected: a native window opens showing the default Tauri React page. Close it (Ctrl-C) once confirmed.

- [ ] **Step 6: Add vitest to the frontend**

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
npm install -D vitest
```
Add to `package.json` `"scripts"`: `"test": "vitest run"`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri v2 React-TS app with vitest"
```

---

## Task 2: Output path naming (`unique_output_path`)

**Files:**
- Create: `src-tauri/src/pdf/mod.rs` (declares `mod naming;`)
- Create: `src-tauri/src/pdf/naming.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod pdf;`)

**Interfaces:**
- Produces: `pub fn unique_output_path(input: &std::path::Path) -> std::path::PathBuf` — returns a non-existing sibling path `<stem>-unlocked.<ext>`, adding ` (N)` before the extension on collision.

- [ ] **Step 1: Write the failing test**

In `src-tauri/src/pdf/naming.rs`:

```rust
use std::path::{Path, PathBuf};

pub fn unique_output_path(_input: &Path) -> PathBuf {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn basic_name() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("report.pdf");
        assert_eq!(unique_output_path(&input), dir.path().join("report-unlocked.pdf"));
    }

    #[test]
    fn collision_appends_number() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("report.pdf");
        fs::write(dir.path().join("report-unlocked.pdf"), b"x").unwrap();
        assert_eq!(unique_output_path(&input), dir.path().join("report-unlocked (2).pdf"));
        fs::write(dir.path().join("report-unlocked (2).pdf"), b"x").unwrap();
        assert_eq!(unique_output_path(&input), dir.path().join("report-unlocked (3).pdf"));
    }
}
```

Create `src-tauri/src/pdf/mod.rs` with:
```rust
pub mod naming;
```
Add `mod pdf;` to the top of `src-tauri/src/lib.rs`. Add `tempfile = "3"` under `[dev-dependencies]` in `src-tauri/Cargo.toml`.

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd src-tauri && cargo test naming 2>&1 | tail -20
```
Expected: FAIL (`unimplemented!()` panics).

- [ ] **Step 3: Implement `unique_output_path`**

Replace the stub in `naming.rs`:
```rust
pub fn unique_output_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = input.extension().map(|e| e.to_string_lossy().into_owned());
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let base = format!("{stem}-unlocked");

    let candidate = |n: usize| -> PathBuf {
        let name = if n == 1 { base.clone() } else { format!("{base} ({n})") };
        match &ext {
            Some(e) => parent.join(format!("{name}.{e}")),
            None => parent.join(name),
        }
    };

    let mut n = 1;
    loop {
        let path = candidate(n);
        if !path.exists() {
            return path;
        }
        n += 1;
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd src-tauri && cargo test naming 2>&1 | tail -20
```
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pdf src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: unique_output_path with collision handling"
```

---

## Task 3: Fixture generation helper

**Files:**
- Create: `src-tauri/tests/common/mod.rs`

**Interfaces:**
- Produces: `pub struct Fixtures { pub plain: PathBuf, pub aes256: PathBuf, pub aes128: PathBuf, pub rc4_128: PathBuf, pub corrupt: PathBuf }` and `pub fn make_fixtures() -> (tempfile::TempDir, Fixtures)`. The `TempDir` must be held by the caller for the lifetime of the test (dropping it deletes the files). Password baked into every encrypted fixture is `secret`.

- [ ] **Step 1: Write the helper**

In `src-tauri/tests/common/mod.rs`:
```rust
use std::path::PathBuf;
use std::process::Command;

pub struct Fixtures {
    pub plain: PathBuf,
    pub aes256: PathBuf,
    pub aes128: PathBuf,
    pub rc4_128: PathBuf,
    pub corrupt: PathBuf,
}

fn qpdf(args: &[&str]) {
    let status = Command::new("qpdf").args(args).status().expect("qpdf must be installed for tests");
    // qpdf exit 0 = ok, 3 = warnings-only (still produced output)
    let code = status.code().unwrap_or(-1);
    assert!(code == 0 || code == 3, "qpdf {args:?} failed with {code}");
}

pub fn make_fixtures() -> (tempfile::TempDir, Fixtures) {
    let dir = tempfile::tempdir().unwrap();
    let p = |name: &str| dir.path().join(name).to_string_lossy().into_owned();

    // Rough minimal PDF, then let qpdf normalize it into a clean base.
    let raw = dir.path().join("raw.pdf");
    std::fs::write(&raw,
        b"%PDF-1.4\n\
          1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
          2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
          3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj\n\
          trailer<</Size 4/Root 1 0 R>>\n%%EOF\n").unwrap();
    qpdf(&[raw.to_str().unwrap(), &p("plain.pdf")]);

    let plain = p("plain.pdf");
    qpdf(&["--encrypt", "secret", "secret", "256", "--", &plain, &p("aes256.pdf")]);
    qpdf(&["--encrypt", "secret", "secret", "128", "--use-aes=y", "--", &plain, &p("aes128.pdf")]);
    qpdf(&["--encrypt", "secret", "secret", "128", "--use-aes=n", "--", &plain, &p("rc4_128.pdf")]);

    let corrupt = dir.path().join("corrupt.pdf");
    std::fs::write(&corrupt, b"%PDF-1.4\nthis is not a valid pdf body at all\n").unwrap();

    let f = Fixtures {
        plain: dir.path().join("plain.pdf"),
        aes256: dir.path().join("aes256.pdf"),
        aes128: dir.path().join("aes128.pdf"),
        rc4_128: dir.path().join("rc4_128.pdf"),
        corrupt,
    };
    (dir, f)
}
```

- [ ] **Step 2: Sanity-check the helper compiles and fixtures generate**

Add a temporary throwaway test file `src-tauri/tests/unlock.rs`:
```rust
mod common;
#[test]
fn fixtures_generate() {
    let (_dir, f) = common::make_fixtures();
    assert!(f.aes256.exists() && f.plain.exists() && f.rc4_128.exists());
}
```

```bash
cd src-tauri && cargo test --test unlock fixtures_generate 2>&1 | tail -20
```
Expected: PASS. (This test file is expanded in Task 6.)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests
git commit -m "test: qpdf fixture generation helper"
```

---

## Task 4: qpdf resolution + encryption detection (`resolve_qpdf`, `is_encrypted`)

**Files:**
- Create: `src-tauri/src/pdf/qpdf.rs`
- Modify: `src-tauri/src/pdf/mod.rs` (add `pub mod qpdf;`)
- Modify: `src-tauri/tests/unlock.rs`

**Interfaces:**
- Produces:
  - `pub fn resolve_qpdf() -> std::path::PathBuf` — returns `PDFUNLOCK_QPDF` env value if set, else a sidecar named `qpdf` next to the current executable if it exists, else the bare name `qpdf` (PATH lookup).
  - `pub fn is_encrypted(qpdf: &Path, input: &Path) -> Result<bool, EngineError>` — runs `qpdf --is-encrypted <input>`; exit 0 → `Ok(true)`, exit 2 → `Ok(false)`, anything else → `Err(EngineError(stderr))`.
  - `pub struct EngineError(pub String)` (derive `Debug`).

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/tests/unlock.rs`:
```rust
use pdfunlock_lib::pdf::qpdf::{is_encrypted, resolve_qpdf};

#[test]
fn detects_encrypted_and_plain() {
    let (_dir, f) = common::make_fixtures();
    let q = resolve_qpdf();
    assert_eq!(is_encrypted(&q, &f.aes256).unwrap(), true);
    assert_eq!(is_encrypted(&q, &f.plain).unwrap(), false);
}
```

> Note the crate name is `pdfunlock_lib` — Tauri's scaffold names the library crate `<app>_lib`. Confirm the actual name in `src-tauri/Cargo.toml` under `[lib] name` and use it consistently. Ensure `pub mod pdf;` (not `mod pdf;`) in `lib.rs` so tests can reach it.

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd src-tauri && cargo test --test unlock detects_encrypted 2>&1 | tail -20
```
Expected: FAIL (unresolved `qpdf` module / functions).

- [ ] **Step 3: Implement `qpdf.rs`**

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct EngineError(pub String);

pub fn resolve_qpdf() -> PathBuf {
    if let Ok(p) = std::env::var("PDFUNLOCK_QPDF") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sidecar = dir.join("qpdf");
            if sidecar.exists() {
                return sidecar;
            }
        }
    }
    PathBuf::from("qpdf")
}

pub fn is_encrypted(qpdf: &Path, input: &Path) -> Result<bool, EngineError> {
    let out = Command::new(qpdf)
        .arg("--is-encrypted")
        .arg(input)
        .output()
        .map_err(|e| EngineError(format!("failed to run qpdf: {e}")))?;
    match out.status.code() {
        Some(0) => Ok(true),
        Some(2) => Ok(false),
        _ => Err(EngineError(String::from_utf8_lossy(&out.stderr).into_owned())),
    }
}
```
Add `pub mod qpdf;` to `src-tauri/src/pdf/mod.rs`.

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd src-tauri && cargo test --test unlock detects_encrypted 2>&1 | tail -20
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pdf src-tauri/tests/unlock.rs
git commit -m "feat: qpdf resolution and encryption detection"
```

---

## Task 5: qpdf decryption (`decrypt`)

**Files:**
- Modify: `src-tauri/src/pdf/qpdf.rs`
- Modify: `src-tauri/tests/unlock.rs`

**Interfaces:**
- Produces:
  - `pub enum DecryptError { WrongPassword, Corrupt, Engine(String) }` (derive `Debug`).
  - `pub fn decrypt(qpdf: &Path, input: &Path, output: &Path, password: &str) -> Result<(), DecryptError>` — runs `qpdf --warning-exit-0 --password-file=- --decrypt <input> <output>` with `password\n` written to stdin. Exit 0 → `Ok`. Exit 2 with stderr containing "invalid password" (case-insensitive) → `WrongPassword`. Exit 2 with stderr containing "not a pdf" or "damaged" or "unable to recover" → `Corrupt`. Any other failure → `Engine(stderr)`.

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/tests/unlock.rs`:
```rust
use pdfunlock_lib::pdf::qpdf::{decrypt, DecryptError};

#[test]
fn decrypt_all_schemes_with_correct_password() {
    let (dir, f) = common::make_fixtures();
    let q = resolve_qpdf();
    for input in [&f.aes256, &f.aes128, &f.rc4_128] {
        let out = dir.path().join("out.pdf");
        decrypt(&q, input, &out, "secret").expect("should decrypt");
        assert!(out.exists());
        // Decrypted output must itself no longer be encrypted.
        assert_eq!(is_encrypted(&q, &out).unwrap(), false);
        std::fs::remove_file(&out).ok();
    }
}

#[test]
fn decrypt_wrong_password() {
    let (dir, f) = common::make_fixtures();
    let q = resolve_qpdf();
    let out = dir.path().join("out.pdf");
    match decrypt(&q, &f.aes256, &out, "nope") {
        Err(DecryptError::WrongPassword) => {}
        other => panic!("expected WrongPassword, got {other:?}"),
    }
}

#[test]
fn decrypt_corrupt_file() {
    let (dir, f) = common::make_fixtures();
    let q = resolve_qpdf();
    let out = dir.path().join("out.pdf");
    match decrypt(&q, &f.corrupt, &out, "secret") {
        Err(DecryptError::Corrupt) | Err(DecryptError::Engine(_)) => {}
        other => panic!("expected Corrupt/Engine, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cd src-tauri && cargo test --test unlock decrypt 2>&1 | tail -25
```
Expected: FAIL (unresolved `decrypt` / `DecryptError`).

- [ ] **Step 3: Implement `decrypt`**

Append to `src-tauri/src/pdf/qpdf.rs`:
```rust
use std::io::Write;
use std::process::Stdio;

#[derive(Debug)]
pub enum DecryptError {
    WrongPassword,
    Corrupt,
    Engine(String),
}

pub fn decrypt(qpdf: &Path, input: &Path, output: &Path, password: &str) -> Result<(), DecryptError> {
    let mut child = Command::new(qpdf)
        .arg("--warning-exit-0")
        .arg("--password-file=-")
        .arg("--decrypt")
        .arg(input)
        .arg(output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| DecryptError::Engine(format!("failed to run qpdf: {e}")))?;

    // qpdf --password-file reads the first line, stripping the trailing newline.
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(format!("{password}\n").as_bytes())
        .map_err(|e| DecryptError::Engine(format!("failed to send password: {e}")))?;

    let out = child
        .wait_with_output()
        .map_err(|e| DecryptError::Engine(format!("qpdf did not complete: {e}")))?;

    if out.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    if stderr.contains("invalid password") {
        Err(DecryptError::WrongPassword)
    } else if stderr.contains("not a pdf")
        || stderr.contains("damaged")
        || stderr.contains("unable to recover")
    {
        // Leave no partial output behind.
        let _ = std::fs::remove_file(output);
        Err(DecryptError::Corrupt)
    } else {
        let _ = std::fs::remove_file(output);
        Err(DecryptError::Engine(String::from_utf8_lossy(&out.stderr).into_owned()))
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd src-tauri && cargo test --test unlock decrypt 2>&1 | tail -25
```
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pdf/qpdf.rs src-tauri/tests/unlock.rs
git commit -m "feat: qpdf decrypt with typed errors"
```

---

## Task 6: Pipeline + serializable types (`unlock_pdf_impl`, `UnlockOutcome`, `UnlockError`)

**Files:**
- Modify: `src-tauri/src/pdf/mod.rs`
- Modify: `src-tauri/tests/unlock.rs`
- Modify: `src-tauri/Cargo.toml` (ensure `serde` with `derive` is a dependency — present in scaffold; add `features = ["derive"]` if missing)

**Interfaces:**
- Produces:
  - `#[derive(Serialize)] #[serde(tag = "status")] pub enum UnlockOutcome { Unlocked { output_path: String }, NotEncrypted }`
  - `#[derive(Serialize)] #[serde(tag = "kind")] pub enum UnlockError { WrongPassword, Corrupt, Io { message: String }, Engine { message: String } }`
  - `pub fn unlock_pdf_impl(input: &Path, password: &str) -> Result<UnlockOutcome, UnlockError>` — resolves qpdf, returns `NotEncrypted` if the file isn't encrypted (no output produced), otherwise computes `unique_output_path`, decrypts, and returns `Unlocked { output_path }`. Maps `DecryptError`/`EngineError` to `UnlockError`.

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/tests/unlock.rs`:
```rust
use pdfunlock_lib::pdf::{unlock_pdf_impl, UnlockError, UnlockOutcome};

#[test]
fn pipeline_unlocks_encrypted() {
    let (_dir, f) = common::make_fixtures();
    match unlock_pdf_impl(&f.aes256, "secret").unwrap() {
        UnlockOutcome::Unlocked { output_path } => {
            assert!(std::path::Path::new(&output_path).exists());
            assert!(output_path.ends_with("-unlocked.pdf"));
        }
        other => panic!("expected Unlocked, got {other:?}"),
    }
}

#[test]
fn pipeline_skips_plain() {
    let (_dir, f) = common::make_fixtures();
    assert!(matches!(unlock_pdf_impl(&f.plain, "secret").unwrap(), UnlockOutcome::NotEncrypted));
}

#[test]
fn pipeline_wrong_password_maps_error() {
    let (_dir, f) = common::make_fixtures();
    assert!(matches!(unlock_pdf_impl(&f.aes256, "nope"), Err(UnlockError::WrongPassword)));
}
```
Add `#[derive(Debug)]` to `UnlockOutcome` (needed for the `panic!` formatting above).

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cd src-tauri && cargo test --test unlock pipeline 2>&1 | tail -25
```
Expected: FAIL (unresolved `unlock_pdf_impl` / types).

- [ ] **Step 3: Implement the pipeline**

In `src-tauri/src/pdf/mod.rs`:
```rust
pub mod naming;
pub mod qpdf;

use serde::Serialize;
use std::path::Path;

use naming::unique_output_path;
use qpdf::{decrypt, is_encrypted, resolve_qpdf, DecryptError};

#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum UnlockOutcome {
    Unlocked { output_path: String },
    NotEncrypted,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum UnlockError {
    WrongPassword,
    Corrupt,
    Io { message: String },
    Engine { message: String },
}

pub fn unlock_pdf_impl(input: &Path, password: &str) -> Result<UnlockOutcome, UnlockError> {
    if !input.exists() {
        return Err(UnlockError::Io { message: format!("file not found: {}", input.display()) });
    }
    let qpdf = resolve_qpdf();

    match is_encrypted(&qpdf, input) {
        Ok(false) => return Ok(UnlockOutcome::NotEncrypted),
        Ok(true) => {}
        Err(e) => return Err(UnlockError::Engine { message: e.0 }),
    }

    let output = unique_output_path(input);
    match decrypt(&qpdf, input, &output, password) {
        Ok(()) => Ok(UnlockOutcome::Unlocked { output_path: output.to_string_lossy().into_owned() }),
        Err(DecryptError::WrongPassword) => Err(UnlockError::WrongPassword),
        Err(DecryptError::Corrupt) => Err(UnlockError::Corrupt),
        Err(DecryptError::Engine(m)) => Err(UnlockError::Engine { message: m }),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd src-tauri && cargo test --test unlock 2>&1 | tail -30
```
Expected: PASS (all unlock tests, including earlier ones).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pdf/mod.rs src-tauri/tests/unlock.rs src-tauri/Cargo.toml
git commit -m "feat: unlock pipeline with serializable outcome/error"
```

---

## Task 7: Tauri `unlock_pdf` command + reveal-in-Finder

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` (add opener plugin)

**Interfaces:**
- Produces (to the frontend):
  - Tauri command `unlock_pdf(input_path: String, password: String) -> Result<UnlockOutcome, UnlockError>`.
  - The `tauri-plugin-opener` for "Reveal in Finder" (`revealItemInDir`) and `tauri-plugin-dialog` for the file picker (both from the scaffold or added here).

- [ ] **Step 1: Add plugins**

```bash
cd src-tauri
cargo add tauri-plugin-opener tauri-plugin-dialog
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
cd .. && npm install @tauri-apps/plugin-opener @tauri-apps/plugin-dialog @tauri-apps/api
```

- [ ] **Step 2: Register the command and plugins in `lib.rs`**

Ensure the top of `src-tauri/src/lib.rs` has `pub mod pdf;`, then add:
```rust
use std::path::PathBuf;
use pdf::{unlock_pdf_impl, UnlockError, UnlockOutcome};

#[tauri::command]
fn unlock_pdf(input_path: String, password: String) -> Result<UnlockOutcome, UnlockError> {
    unlock_pdf_impl(&PathBuf::from(input_path), &password)
}
```
In the `run()` builder, register the plugins and handler:
```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![unlock_pdf])
        // ...existing setup...
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
```
(Keep any scaffold-generated commands/handlers; add `unlock_pdf` to the `generate_handler!` list.)

- [ ] **Step 3: Grant permissions in the capabilities file**

In `src-tauri/capabilities/default.json`, add to `"permissions"`:
```json
"opener:allow-reveal-item-in-dir",
"dialog:allow-open"
```

- [ ] **Step 4: Verify it compiles**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```
Expected: builds without errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: expose unlock_pdf command; add opener/dialog plugins"
```

---

## Task 8: Frontend — types, orchestration, and UI

**Files:**
- Create: `src/types.ts`, `src/unlock.ts`, `src/unlock.test.ts`
- Modify: `src/App.tsx`, `src/styles.css` (replace scaffold demo)

**Interfaces:**
- Consumes: the `unlock_pdf` command and `@tauri-apps/plugin-dialog` `open`, `@tauri-apps/plugin-opener` `revealItemInDir`.
- Produces:
  - `types.ts`: `type UnlockOutcome = { status: "Unlocked"; output_path: string } | { status: "NotEncrypted" }` and `type UnlockError = { kind: "WrongPassword" | "Corrupt" | "Io" | "Engine"; message?: string }`.
  - `unlock.ts`: `type Row = { id: string; path: string; name: string; status: "ready" | "working" | "done" | "skipped" | "error"; detail?: string; outputPath?: string }`, `errorMessage(e: UnlockError): string`, and `async function unlockOne(path: string, password: string, invoke: InvokeFn): Promise<Partial<Row>>` where `InvokeFn = (cmd: string, args: Record<string, unknown>) => Promise<unknown>`. `unlockOne` returns the row-patch for a single file and never throws.

- [ ] **Step 1: Write `types.ts`**

```ts
export type UnlockOutcome =
  | { status: "Unlocked"; output_path: string }
  | { status: "NotEncrypted" };

export type UnlockError = {
  kind: "WrongPassword" | "Corrupt" | "Io" | "Engine";
  message?: string;
};
```

- [ ] **Step 2: Write the failing orchestration test**

`src/unlock.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { unlockOne, errorMessage } from "./unlock";

describe("unlockOne", () => {
  it("maps a successful unlock to a done row", async () => {
    const invoke = async () => ({ status: "Unlocked", output_path: "/a/b-unlocked.pdf" });
    const patch = await unlockOne("/a/b.pdf", "pw", invoke);
    expect(patch.status).toBe("done");
    expect(patch.outputPath).toBe("/a/b-unlocked.pdf");
  });

  it("maps NotEncrypted to a skipped row", async () => {
    const invoke = async () => ({ status: "NotEncrypted" });
    const patch = await unlockOne("/a/b.pdf", "pw", invoke);
    expect(patch.status).toBe("skipped");
  });

  it("maps a thrown UnlockError to an error row without throwing", async () => {
    const invoke = async () => { throw { kind: "WrongPassword" }; };
    const patch = await unlockOne("/a/b.pdf", "pw", invoke);
    expect(patch.status).toBe("error");
    expect(patch.detail).toBe("Incorrect password");
  });
});

describe("errorMessage", () => {
  it("has friendly text per kind", () => {
    expect(errorMessage({ kind: "Corrupt" })).toMatch(/read/i);
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
npm test 2>&1 | tail -20
```
Expected: FAIL (`./unlock` has no such exports).

- [ ] **Step 4: Implement `unlock.ts`**

```ts
import type { UnlockError, UnlockOutcome } from "./types";

export type Row = {
  id: string;
  path: string;
  name: string;
  status: "ready" | "working" | "done" | "skipped" | "error";
  detail?: string;
  outputPath?: string;
};

export type InvokeFn = (cmd: string, args: Record<string, unknown>) => Promise<unknown>;

export function errorMessage(e: UnlockError): string {
  switch (e.kind) {
    case "WrongPassword": return "Incorrect password";
    case "Corrupt": return "Could not read PDF";
    case "Io": return e.message ?? "File error";
    case "Engine": return e.message ?? "Unexpected error";
  }
}

export async function unlockOne(
  path: string,
  password: string,
  invoke: InvokeFn,
): Promise<Partial<Row>> {
  try {
    const res = (await invoke("unlock_pdf", { inputPath: path, password })) as UnlockOutcome;
    if (res.status === "Unlocked") {
      return { status: "done", outputPath: res.output_path, detail: "Saved" };
    }
    return { status: "skipped", detail: "Not password-protected — skipped" };
  } catch (raw) {
    return { status: "error", detail: errorMessage(raw as UnlockError) };
  }
}
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
npm test 2>&1 | tail -20
```
Expected: PASS (4 tests).

- [ ] **Step 6: Build the UI in `App.tsx`**

Replace the scaffold `App.tsx` with a component that:
- Holds `rows: Row[]` and `password: string` state.
- **Drop zone + Browse:** a large dashed drop area. On the app window, listen for file drops via `getCurrentWebview().onDragDropEvent` (from `@tauri-apps/api/webview`) and add each `.pdf` path as a `ready` row. "Browse…" calls `open({ multiple: true, filters: [{ name: "PDF", extensions: ["pdf"] }] })` from `@tauri-apps/plugin-dialog` and adds the returned paths.
- **Password field** with a show/hide toggle (`type` toggles `password`/`text`).
- **"Unlock all"** button (disabled when no `ready` rows or password empty): for each `ready` row set status `working`, then `const patch = await unlockOne(row.path, password, invoke)` (import `invoke` from `@tauri-apps/api/core`), then merge the patch into that row. Process sequentially so status updates are visible; a rejected promise cannot occur because `unlockOne` never throws.
- **Row rendering:** filename, a status pill colored per status, and for `done` rows a "Reveal in Finder" button calling `revealItemInDir(row.outputPath!)` from `@tauri-apps/plugin-opener`.
- Use `crypto.randomUUID()` for row ids; dedupe by path when adding.

Keep component logic thin — all decisions about status live in `unlockOne`.

- [ ] **Step 7: Manually verify end-to-end**

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
# Make a test encrypted PDF first:
qpdf --encrypt secret secret 256 -- <any.pdf> /tmp/enc.pdf
npm run tauri dev
```
Drop `/tmp/enc.pdf`, type `secret`, click Unlock all. Expected: row shows ✓ Saved, `/tmp/enc-unlocked.pdf` exists and opens without a password; "Reveal in Finder" opens Finder. Try a wrong password on another copy → "Incorrect password", batch not aborted. Drop an unencrypted PDF → "Not password-protected — skipped".

- [ ] **Step 8: Commit**

```bash
git add src package.json package-lock.json
git commit -m "feat: frontend UI, types, and batch orchestration"
```

---

## Task 9: Visual polish (frontend-design skill)

**Files:**
- Modify: `src/App.tsx`, `src/styles.css`

**Interfaces:**
- Consumes/Produces: no interface changes — visual only.

- [ ] **Step 1: Invoke the frontend-design skill**

Use the `frontend-design` skill to establish an aesthetic direction (this is the "good UI" requirement). Apply it to the existing components/CSS. Cover:
- System light/dark via `@media (prefers-color-scheme: dark)`; readable in both.
- A confident type scale, generous spacing, and a clear visual hierarchy (drop zone → password → action → results).
- Status pills with distinct, accessible colors (ready/working/done/skipped/error).
- Subtle motion on status transitions and drag-over highlight on the drop zone.
- Empty state ("Drop PDFs here") and a clear disabled state for "Unlock all".

- [ ] **Step 2: Re-verify the app still runs and behaves**

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
npm test 2>&1 | tail -5
npm run tauri dev
```
Expected: tests still pass; UI looks polished; the Task 8 Step 7 flow still works.

- [ ] **Step 3: Commit**

```bash
git add src
git commit -m "style: polished UI with frontend-design pass"
```

---

## Task 10: Bundle qpdf as a self-contained sidecar

**Files:**
- Create: `src-tauri/binaries/` (qpdf binary + dylibs), `scripts/vendor-qpdf.sh`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Produces: a distributable `.app` that runs qpdf without a system/brew install. `resolve_qpdf()` (Task 4) already prefers a sidecar named `qpdf` next to the executable.

- [ ] **Step 1: Write `scripts/vendor-qpdf.sh` to vendor qpdf + its dylibs with fixed rpaths**

```bash
#!/usr/bin/env bash
set -euo pipefail
DEST="src-tauri/binaries"
TRIPLE="aarch64-apple-darwin"
mkdir -p "$DEST"
SRC="$(command -v qpdf)"
cp "$SRC" "$DEST/qpdf-$TRIPLE"

# Copy every Homebrew dylib qpdf (transitively) depends on, into the same dir,
# and rewrite load paths to @loader_path so the bundle is self-contained.
copy_deps() {
  local bin="$1"
  otool -L "$bin" | awk 'NR>1 {print $1}' | grep -E '/(opt|usr/local)/' | while read -r dep; do
    local base; base="$(basename "$dep")"
    if [ ! -f "$DEST/$base" ]; then
      cp "$dep" "$DEST/$base"
      chmod u+w "$DEST/$base"
      install_name_tool -id "@loader_path/$base" "$DEST/$base"
      copy_deps "$DEST/$base"
    fi
    install_name_tool -change "$dep" "@loader_path/$base" "$bin"
  done
}
chmod u+w "$DEST/qpdf-$TRIPLE"
copy_deps "$DEST/qpdf-$TRIPLE"
echo "Vendored qpdf and dylibs into $DEST"
```

```bash
chmod +x scripts/vendor-qpdf.sh && ./scripts/vendor-qpdf.sh
otool -L src-tauri/binaries/qpdf-aarch64-apple-darwin   # verify only @loader_path/system libs remain
```

- [ ] **Step 2: Declare the sidecar and resources in `tauri.conf.json`**

Under `"bundle"`:
```json
"externalBin": ["binaries/qpdf"],
"resources": ["binaries/*.dylib"]
```
(Tauri appends the target triple to `externalBin` entries automatically, matching `qpdf-aarch64-apple-darwin`.)

- [ ] **Step 3: Ensure the app finds the sidecar and its dylibs at runtime**

The externalBin is copied next to the app executable, so `resolve_qpdf()` finds `qpdf`. Because the dylibs were rewritten to `@loader_path`, they must sit beside the qpdf binary. Confirm the bundle build places `*.dylib` next to the sidecar; if Tauri stages resources in `Resources/` instead, copy the dylibs next to the sidecar in a `beforeBundleCommand`, or vendor them next to `binaries/qpdf-<triple>` and add that whole dir. Verify empirically in Step 4.

- [ ] **Step 4: Build the release bundle and verify it runs without brew qpdf**

```bash
export PATH="/Users/abhiramnajith/Library/Application Support/Zed/node/node-v24.11.0-darwin-arm64/bin:$PATH"
npm run tauri build 2>&1 | tail -20
# Temporarily hide the system qpdf to prove self-containment:
sudo mv "$(command -v qpdf)" "$(command -v qpdf).hidden"
open "src-tauri/target/release/bundle/macos/pdfunlock.app"
# Test unlocking /tmp/enc.pdf in the launched app, then restore:
sudo mv "$(command -v qpdf).hidden" "$(command -v qpdf)"
```
Expected: the bundled app unlocks a PDF with no system qpdf present.

- [ ] **Step 5: Commit**

```bash
git add scripts/vendor-qpdf.sh src-tauri/tauri.conf.json .gitignore
# Note: binaries/ are build artifacts — either commit them or regenerate via the script in CI.
git commit -m "build: bundle qpdf as self-contained macOS sidecar"
```

---

## Self-Review

**Spec coverage:**
- Unlock with known password → Tasks 5, 6, 7, 8 ✓
- Batch, files share a password → Task 8 (shared password field, per-row processing) ✓
- Save `<stem>-unlocked.<ext>` next to original, never overwrite, ` (N)` collisions → Task 2 ✓
- All encryption schemes (RC4/AES-128/AES-256) → Tasks 3, 5 (fixtures + tests per scheme) ✓
- Typed errors WrongPassword/Corrupt/Io/Engine → Tasks 5, 6 ✓
- NotEncrypted → skip with note → Tasks 6, 8 ✓
- Batch continues on error → Task 8 (`unlockOne` never throws) ✓
- Password not on argv → Task 5 (`--password-file=-` + stdin) ✓
- Reveal in Finder → Task 7, 8 ✓
- Good UI, light/dark → Tasks 8, 9 ✓
- No network → nothing in the plan adds network access ✓
- qpdf sidecar bundling, macOS-first arm64 → Task 10 ✓
- Rust integration tests over fixtures → Tasks 3–6 ✓

**Placeholder scan:** No TBD/TODO; all code steps contain runnable code. Task 10 Step 3 is a genuine verify-then-adjust step (dylib staging location is environment-dependent) rather than a placeholder — it names the exact fallback action.

**Type consistency:** `UnlockOutcome`/`UnlockError` field names (`output_path`, `kind`, `message`, `status`) are consistent between the Rust `#[serde]` definitions (Task 6) and the TS mirrors (Task 8). `resolve_qpdf`/`is_encrypted`/`decrypt`/`unlock_pdf_impl` signatures match across their producing and consuming tasks. Library crate name `pdfunlock_lib` is flagged in Task 4 Step 1 to be confirmed against the actual scaffold and used consistently.
