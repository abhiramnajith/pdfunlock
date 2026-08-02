use std::path::PathBuf;
use std::process::Command;

pub struct Fixtures {
    pub plain: PathBuf,
    pub aes256: PathBuf,
    pub aes128: PathBuf,
    pub rc4_128: PathBuf,
    pub corrupt: PathBuf,
}

fn qpdf(args: &[&str]) {
    let status = Command::new("qpdf").args(args).status().expect("qpdf must be installed for tests");
    // qpdf exit 0 = ok, 3 = warnings-only (still produced output)
    let code = status.code().unwrap_or(-1);
    assert!(code == 0 || code == 3, "qpdf {args:?} failed with {code}");
}

pub fn make_fixtures() -> (tempfile::TempDir, Fixtures) {
    let dir = tempfile::tempdir().unwrap();
    let p = |name: &str| dir.path().join(name).to_string_lossy().into_owned();

    // Rough minimal PDF, then let qpdf normalize it into a clean base.
    let raw = dir.path().join("raw.pdf");
    std::fs::write(&raw,
        b"%PDF-1.4\n\
          1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
          2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
          3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj\n\
          trailer<</Size 4/Root 1 0 R>>\n%%EOF\n").unwrap();
    qpdf(&[raw.to_str().unwrap(), &p("plain.pdf")]);

    let plain = p("plain.pdf");
    qpdf(&["--encrypt", "secret", "secret", "256", "--", &plain, &p("aes256.pdf")]);
    qpdf(&["--encrypt", "secret", "secret", "128", "--use-aes=y", "--", &plain, &p("aes128.pdf")]);
    // qpdf 12+ refuses RC4 unless --allow-weak-crypto is passed as a global option (before --encrypt).
    qpdf(&["--allow-weak-crypto", "--encrypt", "secret", "secret", "128", "--use-aes=n", "--", &plain, &p("rc4_128.pdf")]);

    let corrupt = dir.path().join("corrupt.pdf");
    std::fs::write(&corrupt, b"%PDF-1.4\nthis is not a valid pdf body at all\n").unwrap();

    let f = Fixtures {
        plain: dir.path().join("plain.pdf"),
        aes256: dir.path().join("aes256.pdf"),
        aes128: dir.path().join("aes128.pdf"),
        rc4_128: dir.path().join("rc4_128.pdf"),
        corrupt,
    };
    (dir, f)
}
