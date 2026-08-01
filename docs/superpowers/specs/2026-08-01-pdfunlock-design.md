# PDFUnlock — Design

**Date:** 2026-08-01
**Status:** Approved (pending final spec review)

## Summary

PDFUnlock is a macOS-first desktop app that removes password protection from
PDFs the user already knows the password to. The user supplies the password;
the app decrypts and saves a copy with the password removed. It supports batch
processing of multiple files that share a password.

This is a "unlock with known password" tool. It does **not** attempt to recover,
guess, or brute-force unknown passwords.

## Goals

- Unlock one or more password-protected PDFs given the correct password.
- Save a decrypted copy without ever modifying or overwriting the original.
- A polished, modern UI (drag-and-drop, clear per-file status, light/dark).
- Handle all common PDF encryption schemes reliably.

## Non-Goals

- Password recovery / brute-force / dictionary attacks.
- Editing PDF content, merging, compression, or other PDF manipulation.
- Cloud upload or any network transmission of files. Everything is local.

## Architecture

**Framework:** Tauri (Rust backend + web frontend).

- **Frontend:** React + Vite + TypeScript. Handles the file list, password
  input, and status display. Communicates with the backend via Tauri commands.
- **Backend (Rust):** Exposes a single Tauri command:

  ```
  unlock_pdf(input_path: String, password: String) -> Result<UnlockOutcome, UnlockError>
  ```

  - `UnlockOutcome`:
    - `Unlocked { output_path }` — decrypted copy written.
    - `NotEncrypted` — file has no password; skipped (see Error Handling).
  - `UnlockError` (typed, mapped from the PDF engine):
    - `WrongPassword`
    - `Corrupt`
    - `IoError { message }`
    - `EngineError { message }` — catch-all for unexpected engine failures.

  The frontend calls this command once per file and updates that file's row
  independently, so one failure never halts the batch.

## PDF Engine

**Decision: bundle `qpdf` as a Tauri sidecar binary.**

The Rust backend invokes qpdf as a subprocess:

```
qpdf --password=<PASSWORD> --decrypt <input.pdf> <output.pdf>
```

**Rationale:** qpdf is the reference-quality tool for this task and handles
every standard PDF encryption scheme — RC4 (40/128-bit), AES-128, and
AES-256/R6. Modern protected PDFs predominantly use AES-256, which pure-Rust
crates (e.g. `lopdf`) handle unreliably or not at all. Correctness of unlocking
is the core value of the app, so reliability wins over avoiding a bundled binary.

**Trade-off:** We bundle a qpdf binary (~2 MB) per target platform via Tauri's
sidecar/`externalBin` mechanism. For the macOS-first release that is a single
binary (arm64; add x64/universal as needed). Adding Windows/Linux later is
purely a matter of bundling those platforms' binaries — no code change.

**Error mapping:** qpdf exit codes and stderr map to `UnlockError` variants.
Notably, a wrong password yields a distinct, detectable failure that maps to
`WrongPassword` rather than a generic error.

**Password handling:** The password is passed to qpdf without exposing it on a
shared shell command line where practical (prefer argument passing that avoids
process-list leakage, e.g. `@filename`/stdin mechanisms qpdf supports, to be
finalized in implementation).

## Output Behavior

- Decrypted copy is saved **in the same folder as the original**, with
  `-unlocked` appended before the extension: `report.pdf` → `report-unlocked.pdf`.
- Originals are **never** modified or overwritten.
- If the target name already exists, append ` (2)`, ` (3)`, etc.:
  `report-unlocked (2).pdf`.

## UI / UX

The "good UI" goal is a primary requirement. The frontend-design skill will be
invoked during implementation for visual polish.

- **Drop zone:** Large drag-and-drop target; also a "Browse…" button using the
  native file picker (multi-select).
- **File list:** Each dropped PDF appears as a row with file name, size, and a
  status pill:
  - `Ready` → `Unlocking…` → `✓ Saved` / `✗ error message`.
- **Password field:** One shared password input (files often share a password),
  with a show/hide toggle.
- **Action:** "Unlock all" button processes every `Ready` file.
- **Results:** A successful row shows the saved file and a "Reveal in Finder"
  link (Tauri opener).
- **Theme:** Follows system light/dark. Generous spacing, keyboard-friendly,
  subtle motion for state transitions.

## Error Handling

- **Wrong password:** Row shows "Incorrect password." The batch continues;
  other files are unaffected.
- **Not encrypted:** File is **skipped** with an informational note
  ("Not password-protected — skipped"). No `-unlocked` copy is produced.
- **Corrupt / unreadable:** Row shows "Could not read PDF."
- **I/O errors** (e.g. no write permission): Row shows a clear message.
- No error ever aborts the overall batch.

## Testing

- **Rust integration tests** exercise `unlock_pdf` against fixture PDFs:
  - correct password (AES-256, AES-128, RC4) → `Unlocked`
  - wrong password → `WrongPassword`
  - unencrypted file → `NotEncrypted`
  - corrupt file → `Corrupt`
  - output naming/collision behavior (`-unlocked`, ` (2)`)
  - originals unchanged after run
- **Fixtures** are generated in a test setup step using qpdf (encrypting a
  known sample PDF under each scheme) so the repo need not commit many binaries.
- Frontend logic (status transitions, batch orchestration) covered by unit
  tests where practical.

## Platform Scope

- **v1:** macOS (Apple Silicon primary). qpdf sidecar bundled for macOS.
- **Later:** Windows and Linux by bundling their qpdf binaries; no logic change.
