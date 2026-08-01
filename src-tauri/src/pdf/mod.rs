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
