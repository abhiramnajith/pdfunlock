use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;
use std::process::Stdio;

#[derive(Debug)]
pub struct EngineError(pub String);

/// Build a human-readable engine error that is *never* empty. A DLL/.so loader
/// failure (or any crash before qpdf writes to stderr) exits non-zero with no
/// stderr, so we always fold in the exit code — otherwise the UI shows a blank
/// error and the user has nothing to report.
fn engine_message(stderr: &[u8], status: std::process::ExitStatus) -> String {
    let msg = String::from_utf8_lossy(stderr).trim().to_string();
    let code = status
        .code()
        .map(|c| format!("exit code {c} (0x{c:08X})"))
        .unwrap_or_else(|| "terminated by signal".into());
    if msg.is_empty() {
        format!("qpdf failed: {code}")
    } else {
        format!("{msg} ({code})")
    }
}

/// Filename of the bundled qpdf sidecar. Namespaced as `pdfunlock-qpdf` so the
/// Linux packages don't install a bare `/usr/bin/qpdf` that collides with the
/// distro `qpdf` package. Windows executables carry a `.exe` extension, and
/// Rust does not auto-append it to an explicit path (unlike a bare-name PATH
/// lookup, which does via PATHEXT).
fn sidecar_qpdf_name() -> &'static str {
    if cfg!(windows) { "pdfunlock-qpdf.exe" } else { "pdfunlock-qpdf" }
}

pub fn resolve_qpdf() -> PathBuf {
    if let Ok(p) = std::env::var("PDFUNLOCK_QPDF") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sidecar = dir.join(sidecar_qpdf_name());
            if sidecar.exists() {
                return sidecar;
            }
        }
    }
    PathBuf::from("qpdf")
}

/// Construct a `Command` for the qpdf sidecar with the platform tweaks it needs.
fn qpdf_command(qpdf: &Path) -> Command {
    // `mut` is only exercised on Windows (the block below is cfg'd out elsewhere).
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = Command::new(qpdf);
    // On Windows the child process inherits PATH as part of the DLL search
    // order, so prepend the sidecar's own directory. This guarantees qpdf.exe
    // finds qpdf30.dll + the MSVC runtime that sit beside it, independent of
    // how the bundler laid out `resources` vs `externalBin`.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Don't flash a console window for each qpdf invocation.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Some(dir) = qpdf.parent() {
            let existing = std::env::var("PATH").unwrap_or_default();
            cmd.env("PATH", format!("{};{}", dir.display(), existing));
        }
    }
    cmd
}

pub fn is_encrypted(qpdf: &Path, input: &Path) -> Result<bool, EngineError> {
    let out = qpdf_command(qpdf)
        .arg("--is-encrypted")
        .arg(input)
        .output()
        .map_err(|e| EngineError(format!("failed to run qpdf: {e}")))?;
    match out.status.code() {
        Some(0) => Ok(true),
        Some(2) => Ok(false),
        _ => Err(EngineError(engine_message(&out.stderr, out.status))),
    }
}

#[derive(Debug)]
pub enum DecryptError {
    WrongPassword,
    Corrupt,
    Engine(String),
}

pub fn decrypt(qpdf: &Path, input: &Path, output: &Path, password: &str) -> Result<(), DecryptError> {
    let mut child = qpdf_command(qpdf)
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
    if let Err(e) = child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(format!("{password}\n").as_bytes())
    {
        // Reap the child so a failed write can't leave a zombie behind.
        let _ = child.wait();
        return Err(DecryptError::Engine(format!("failed to send password: {e}")));
    }

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
        Err(DecryptError::Engine(engine_message(&out.stderr, out.status)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_binary_name_is_platform_correct() {
        assert_eq!(
            sidecar_qpdf_name(),
            if cfg!(windows) { "pdfunlock-qpdf.exe" } else { "pdfunlock-qpdf" }
        );
    }

    // A non-zero exit with EMPTY stderr (e.g. a DLL/.so loader failure) must
    // still yield a message the user can see and report.
    #[cfg(unix)]
    #[test]
    fn engine_message_is_never_empty_on_failure() {
        let out = Command::new("sh").args(["-c", "exit 7"]).output().unwrap();
        assert!(!out.status.success());
        let msg = engine_message(&out.stderr, out.status);
        assert!(!msg.trim().is_empty(), "message was empty");
        assert!(msg.contains("exit code 7"), "got: {msg}");
    }
}
