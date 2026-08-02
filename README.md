# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Building a release bundle

The release `.app` ships qpdf as a self-contained sidecar (no system/brew
qpdf required at runtime). To build it, the build machine needs:

- `brew install qpdf`
- `python3` available on PATH

`npm run tauri build` runs `scripts/vendor-qpdf.sh` automatically (wired in
as `beforeBuildCommand`), which vendors qpdf and its non-system dylibs into
`src-tauri/binaries/` (gitignored — regenerated on every build, not checked
into the repo).
