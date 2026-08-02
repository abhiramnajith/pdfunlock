# Cross-Platform Builds + Hardened Auto-Release — Design

**Date:** 2026-08-02
**Status:** Approved (pending final spec review)
**Builds on:** the shipped v0.1 app (`docs/superpowers/specs/2026-08-01-pdfunlock-design.md`)

## Summary

PDFUnlock currently builds and runs on macOS (arm64) only, is installed by
building from source, and still ships the default Tauri/Vite branding under an
MIT license. This effort makes it a genuinely cross-platform, downloadable
product:

- Build for **five targets**: macOS arm64, macOS x86-64, Linux x86-64, Linux
  arm64, Windows x86-64.
- **Remove the "build it yourself" step** by publishing prebuilt installers to
  GitHub Releases via a **security-hardened GitHub Actions** pipeline.
- Replace the license with **MIT + Commons Clause** (free to use/modify/share,
  no selling without permission; owner retains copyright).
- Replace the default **Tauri/Vite/React branding** with a custom PDFUnlock
  icon and favicon.

## Goals

- One `git tag v*` push produces downloadable installers for all five targets,
  attached to a GitHub Release, with integrity metadata.
- Each installer is self-contained (bundles qpdf) so end users install nothing
  extra.
- The CI/CD follows current supply-chain security best practices appropriate
  for a **public** repository.
- The app no longer ships anyone else's logo/trademark.

## Non-Goals (explicitly out of scope for this effort)

- **Code signing / notarization** on macOS and Windows. The apps ship unsigned;
  users click through Gatekeeper (right-click → Open) / SmartScreen (More info →
  Run anyway). Verification is provided via checksums + provenance instead.
  Revisit if the app is distributed widely.
- Windows on ARM64 and a macOS universal binary (per-arch installers only).
- Any change to the app's decryption behavior or UI (that is v0.1, already
  shipped).
- Publishing to app stores or package repositories (Homebrew, winget, Flatpak).

## Platform / Architecture Matrix

| OS      | Arch   | Runner            | Artifacts                     |
|---------|--------|-------------------|-------------------------------|
| macOS   | arm64  | `macos-14`        | `.dmg`                        |
| macOS   | x86-64 | `macos-13`        | `.dmg`                        |
| Linux   | x86-64 | `ubuntu-22.04`    | `.AppImage`, `.deb`, `.rpm`   |
| Linux   | arm64  | `ubuntu-24.04-arm`| `.AppImage`, `.deb`, `.rpm`   |
| Windows | x86-64 | `windows-latest`  | `.msi`, `.exe` (NSIS)         |

`ubuntu-22.04` is chosen for x86-64 Linux for the widest glibc compatibility of
the produced AppImage. If `ubuntu-24.04-arm` public-repo runners are
unavailable at implementation time, Linux arm64 is dropped to a documented
follow-up rather than blocking the rest.

## qpdf Sidecar Per-OS

The app resolves qpdf via `resolve_qpdf()` (env `PDFUNLOCK_QPDF` → a `qpdf`
binary next to the executable → PATH). Tauri's `externalBin` places a
per-target binary next to the app executable on every platform. Each OS needs
its own vendoring of qpdf + its non-system libraries into `src-tauri/binaries/`.

- **macOS** — existing `scripts/vendor-qpdf.sh` renamed to
  `scripts/vendor-qpdf-macos.sh` (logic unchanged: copy dylibs, rewrite load
  commands with `install_name_tool`, dual `LC_RPATH`, ad-hoc re-sign).
- **Linux** — new `scripts/vendor-qpdf-linux.sh`: copy `qpdf` + its non-system
  `.so` dependencies (resolve via `ldd`, exclude the standard system set —
  libc, libm, libdl, libpthread, ld-linux, etc.), and set `RPATH=$ORIGIN` with
  `patchelf` so the binary finds its bundled libs beside itself.
- **Windows** — new `scripts/vendor-qpdf-windows.ps1` (or bash for Git Bash):
  download the **official qpdf Windows release zip** from the qpdf GitHub
  releases, verify its **published SHA-256** before extracting, and copy
  `qpdf.exe` + its DLLs into `src-tauri/binaries/`. Windows resolves DLLs from
  the executable's own directory, so no rpath work is required.
- **Dispatcher** — `scripts/prepare-qpdf-sidecar.sh` branches on `uname -s`
  (`Darwin`/`Linux`/`MINGW*|MSYS*` for Git Bash on Windows) and calls the right
  vendor script. This is wired as `beforeBuildCommand` in `tauri.conf.json`, so
  both local builds and CI vendor the correct sidecar automatically.

**Rust change:** `resolve_qpdf()` gets a `#[cfg(windows)]` arm that looks for
`qpdf.exe` (Rust does not auto-append `.exe` to an explicit path). All other
platforms keep `qpdf`.

**Tauri config:** `externalBin` continues to reference `binaries/qpdf` (Tauri
appends the target triple); `resources` includes the platform libs
(`binaries/*.dylib` on macOS, `binaries/*.so*` on Linux, `binaries/*.dll` on
Windows — a single glob list is fine since only the matching files exist per
build).

**Linux staging risk + fallback:** making `$ORIGIN` resolve to wherever Tauri
places the `.so`s inside each of AppImage/.deb/.rpm is the fiddliest part and
is **CI-tested only** (development happens on macOS). If `$ORIGIN` staging
proves unreliable within the CI budget, the documented fallback is: for
`.deb`/`.rpm`, declare a package dependency on the distro `qpdf` package (via
Tauri's `bundle.linux.deb.depends` / `.rpm.depends`) and bundle qpdf only for
the AppImage. This fallback is a last resort, chosen only if uniform bundling
fails in CI.

## CI/CD — Security-Hardened GitHub Actions

Two workflows under `.github/workflows/`.

### Shared hardening requirements (both workflows)

- **Least-privilege token:** every workflow sets top-level
  `permissions: contents: read`. Individual jobs elevate only what they need
  (the release job adds `contents: write`, `id-token: write`,
  `attestations: write`).
- **Pin all third-party actions to a full 40-char commit SHA** (with a
  trailing comment naming the human version). No `@v4`/`@main` references.
- **`step-security/harden-runner`** as the first step of every job, in `audit`
  egress mode.
- **`actions/checkout` with `persist-credentials: false`** on build jobs.
- **`concurrency`** groups keyed on workflow + ref, cancelling superseded runs.
- **Explicit `timeout-minutes`** on every job.
- **Pinned toolchains:** a fixed Rust toolchain (stable, pinned version) and a
  pinned Node version; dependency installs use `npm ci` (lockfile-exact) and
  `cargo` builds pass `--locked`.
- **No `pull_request_target`.** Nothing that runs untrusted fork code is ever
  granted secrets or write permissions.

### `ci.yml` — tests & scanning (on pushes to `main` and PRs)

- Runs on a single Linux runner (plus macOS for the Rust tests that need qpdf).
- Steps: install qpdf, `npm ci`, `npm test` (vitest), `cargo test`,
  `npm audit --omit=dev` (report), `cargo audit`.
- **CodeQL** analysis for JavaScript/TypeScript.
- Read-only token; no secrets consumed; safe for fork PRs.

### `release.yml` — matrix build & publish (on `v*` tags only)

- **Trigger:** `push: tags: ['v*']` (never on PRs).
- **Matrix:** the five targets above, using `tauri-apps/tauri-action` (SHA-
  pinned). Each runner installs its prerequisites (qpdf + patchelf +
  AppImage/webkit deps on Linux; qpdf fetched by the Windows vendor script;
  qpdf via Homebrew on macOS), runs the build (which vendors the sidecar via
  `beforeBuildCommand`), and produces its installers.
- **Integrity metadata for every artifact:**
  - **SHA-256 checksums** generated and uploaded (a `SHA256SUMS` file on the
    release).
  - **Build provenance attestations** via `actions/attest-build-provenance`
    (requires `id-token: write` + `attestations: write`), so downloads can be
    verified against the workflow that built them — the integrity substitute
    for the absent code signature.
- **Release creation:** a draft/published GitHub Release for the tag with all
  installers + `SHA256SUMS` attached. `tauri-action` handles release creation;
  the release job runs inside a protected GitHub **Environment** named
  `release`.
- **Repository protections (documented, applied via GitHub settings, not
  code):** tag protection rule for `v*`, branch protection on `main` requiring
  `ci.yml` to pass, and required review on the `release` environment.

### Dependabot

`.github/dependabot.yml` covering three ecosystems: `github-actions`, `npm`
(root), and `cargo` (`src-tauri`), weekly — so SHA-pinned actions and
dependencies stay patched.

## License — MIT + Commons Clause

- Replace `LICENSE` with **MIT + Commons Clause**: the MIT text plus the
  Commons Clause rider, which removes the right to "Sell" the software (provide
  it, or a product/service whose value derives substantially from it, to third
  parties for a fee) without the owner's permission. Copyright holder:
  `abhiramnajith`.
- `src-tauri/Cargo.toml`: drop the SPDX `license` field (this combination has
  no standard SPDX expression) and use `license-file = "../LICENSE"` — or the
  crate-appropriate relative path.
- `package.json`: set `"license": "SEE LICENSE IN LICENSE"`.
- README: a **License** section stating the terms in plain language (free to
  use/modify/share; no selling without permission; contact for a commercial
  license) plus the not-legal-advice note.
- This is **source-available, not OSI open source** — an intentional choice.

## Branding — Custom Icon

- Design a **PDFUnlock app icon**: a padlock motif in the app's
  petrol-teal-on-porcelain palette, consistent with the in-app hero glyph, as a
  1024×1024 source image (`src-tauri/icon-source.png`, kept in the repo for
  future regeneration).
- Generate the full icon set with `tauri icon <source>` — overwrites every
  file in `src-tauri/icons/` (macOS `.icns`, Windows `.ico` + `Square*Logo`
  PNGs, and the PNG sizes referenced in `tauri.conf.json`).
- Create a matching **favicon** (reuse the padlock as an SVG in `public/`) and
  update `index.html`'s `<link rel="icon">` to point at it.
- **Delete** the template logos: `public/tauri.svg`, `public/vite.svg`,
  `src/assets/react.svg` (and remove `src/assets/` if it becomes empty).
- The rendered icon is shown to the owner for approval before it is locked in.

## First Release + Documentation

- After CI is green, tag **`v0.1.0`** to fire `release.yml` and produce the
  first public release.
- Rewrite README **Install** to lead with **Download from Releases**: a
  per-OS table with the file to grab, the macOS right-click→Open note, the
  Windows SmartScreen note, `chmod +x` for the Linux AppImage, and **how to
  verify** the download (SHA-256 against `SHA256SUMS`, and provenance via
  `gh attestation verify`). Demote "build from source" to a secondary section.

## Testing / Verification Strategy

- **macOS build** is verifiable locally (developer machine): the release build
  produces working per-arch `.dmg`s and the in-bundle qpdf runs (as in v0.1).
- **Linux and Windows builds are CI-tested only.** Success criteria: the
  `release.yml` matrix completes, every expected artifact is attached to the
  release, and — where feasible — a CI smoke step runs the bundled qpdf
  (`qpdf --version`) inside the built package to confirm its libraries resolve.
- **Existing test suites** (8 Rust integration tests + 4 vitest) continue to
  run in `ci.yml` and must stay green.
- **Security posture** is verified by inspection: actions SHA-pinned,
  permissions minimal, no secrets on PR triggers, provenance + checksums
  present on the release.

## Risks

- **Linux `.so` staging** (see fallback above) — the highest-uncertainty item.
- **`ubuntu-24.04-arm` runner availability** — Linux arm64 dropped to a
  follow-up if unavailable.
- **Unsigned binaries** — Gatekeeper/SmartScreen friction is accepted and
  documented; checksums + provenance mitigate integrity concerns.
- **Commons Clause fuzziness** — the "derives substantially" wording is known
  to be imprecise; acceptable for this project, lawyer review recommended
  before any enforcement.
