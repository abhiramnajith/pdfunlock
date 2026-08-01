use std::path::{Path, PathBuf};

pub fn unique_output_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = input.extension().map(|e| e.to_string_lossy().into_owned());
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let base = format!("{stem}-unlocked");

    let candidate = |n: usize| -> PathBuf {
        let name = if n == 1 { base.clone() } else { format!("{base} ({n})") };
        match &ext {
            Some(e) => parent.join(format!("{name}.{e}")),
            None => parent.join(name),
        }
    };

    let mut n = 1;
    loop {
        let path = candidate(n);
        if !path.exists() {
            return path;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn basic_name() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("report.pdf");
        assert_eq!(unique_output_path(&input), dir.path().join("report-unlocked.pdf"));
    }

    #[test]
    fn collision_appends_number() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("report.pdf");
        fs::write(dir.path().join("report-unlocked.pdf"), b"x").unwrap();
        assert_eq!(unique_output_path(&input), dir.path().join("report-unlocked (2).pdf"));
        fs::write(dir.path().join("report-unlocked (2).pdf"), b"x").unwrap();
        assert_eq!(unique_output_path(&input), dir.path().join("report-unlocked (3).pdf"));
    }
}
