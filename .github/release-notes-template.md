<!-- Release notes template. __VERSION__ is replaced with the tag version (without the leading v) by release.yml. -->
**PDFUnlock** removes password protection from PDFs you know the password to — batch, drag-and-drop, entirely on your device (no upload). qpdf is bundled inside every download, so there's nothing else to install.

> Builds are **not code-signed**, so each OS shows a one-time "unrecognized developer" prompt on first launch — steps to get past it are below. You can verify any download against `SHA256SUMS` (attached) and its build-provenance attestation.

---

## macOS

**Download:**
- Apple Silicon (M-series): `pdfunlock___VERSION___aarch64.dmg`
- Intel: `pdfunlock___VERSION___x64.dmg`

*(Not sure? Apple menu →  About This Mac. "Apple M…" = Apple Silicon.)*

**Install:** open the `.dmg`, drag **PDFUnlock** into **Applications**.

**First launch:** double-clicking shows *"cannot be opened — unidentified developer."* Instead:
- **Right-click** the app → **Open** → **Open** (only needed once), or
- **System Settings → Privacy & Security** → scroll down → **Open Anyway**.

## Windows (x64)

**Download:** `pdfunlock___VERSION___x64-setup.exe` (installer, recommended) or `pdfunlock___VERSION___x64_en-US.msi`.

**Install:** run the file and follow the prompts.

**First launch:** if SmartScreen shows *"Windows protected your PC,"* click **More info → Run anyway**.

## Linux

**Requires Ubuntu 22.04+, Debian 12+, or Fedora 36+** (glibc 2.35+ and webkit2gtk-4.1). Older distros like Ubuntu 20.04 are not supported.

Pick the file for your CPU architecture (`uname -m`):

| `uname -m` | Download |
|---|---|
| `x86_64` | `*_amd64.deb` · `*_amd64.AppImage` · `*.x86_64.rpm` |
| `aarch64` | `*_arm64.deb` · `*_aarch64.AppImage` · `*.aarch64.rpm` |

Then pick the format for your distro:

**AppImage** (most distros, no install):
```bash
chmod +x pdfunlock___VERSION___amd64.AppImage
./pdfunlock___VERSION___amd64.AppImage
```
If it complains about FUSE on newer Ubuntu: `sudo apt install libfuse2t64` (or `libfuse2`).

**Debian / Ubuntu** (`.deb`):
```bash
sudo apt install ./pdfunlock___VERSION___amd64.deb
```

**Fedora / RHEL / openSUSE** (`.rpm`):
```bash
sudo dnf install ./pdfunlock-__VERSION__-1.x86_64.rpm
```

---

## Verify your download (optional)

```bash
# Checksum — compare against the SHA256SUMS file on this release
sha256sum <downloaded-file>        # Linux
shasum -a 256 <downloaded-file>    # macOS

# Provenance — proves the file was built by this repo's Release workflow
gh attestation verify <downloaded-file> --repo abhiramnajith/pdfunlock
```

## Using it

1. Open PDFUnlock.
2. Drag one or more password-protected PDFs onto the drop zone (or click **Browse**).
3. Enter the password the files share.
4. Click **Unlock all** — each file is saved as `name-unlocked.pdf` next to the original (originals are never modified).
