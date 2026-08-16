use std::path::{Component, Path, Prefix};

/// Accepts normal and Windows verbatim paths only when their canonical drive is D.
/// Callers must canonicalize first so `..`, links and relative drive paths are resolved.
pub fn is_canonical_drive_d(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) if letter.eq_ignore_ascii_case(&b'D'))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_standard_and_long_d_drive_paths() {
        assert!(is_canonical_drive_d(Path::new(r"D:\IB Past Papers\paper.pdf")));
        assert!(is_canonical_drive_d(Path::new(r"\\?\D:\IB Past Papers\very-long-path\paper.pdf")));
    }

    #[test]
    fn rejects_other_drives_and_network_paths() {
        assert!(!is_canonical_drive_d(Path::new(r"C:\Temp\paper.pdf")));
        assert!(!is_canonical_drive_d(Path::new(r"\\server\share\paper.pdf")));
    }
}
