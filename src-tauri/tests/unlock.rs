mod common;
#[test]
fn fixtures_generate() {
    let (_dir, f) = common::make_fixtures();
    assert!(f.aes256.exists() && f.plain.exists() && f.rc4_128.exists());
}
