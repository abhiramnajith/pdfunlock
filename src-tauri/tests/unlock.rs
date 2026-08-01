mod common;
use pdfunlock_lib::pdf::qpdf::{is_encrypted, resolve_qpdf, decrypt, DecryptError};

#[test]
fn fixtures_generate() {
    let (_dir, f) = common::make_fixtures();
    assert!(f.aes256.exists() && f.plain.exists() && f.rc4_128.exists());
}

#[test]
fn detects_encrypted_and_plain() {
    let (_dir, f) = common::make_fixtures();
    let q = resolve_qpdf();
    assert_eq!(is_encrypted(&q, &f.aes256).unwrap(), true);
    assert_eq!(is_encrypted(&q, &f.plain).unwrap(), false);
}

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
