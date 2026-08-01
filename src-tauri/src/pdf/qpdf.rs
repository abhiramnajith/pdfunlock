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
