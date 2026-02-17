use std::io;
use std::path::Path;

/// Atomic file write: writes data to a temporary sibling file, then renames
/// into place. Rename is atomic on POSIX when src and dst are on the same
/// filesystem (guaranteed here — sibling file).
pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");

        atomic_write(&path, b"hello").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello");

        // Temp file should not linger
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");

        std::fs::write(&path, "old").unwrap();
        atomic_write(&path, b"new").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "new");
    }
}
