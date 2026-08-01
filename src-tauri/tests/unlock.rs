mod common;
use pdfunlock_lib::pdf::qpdf::{is_encrypted, resolve_qpdf};

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
