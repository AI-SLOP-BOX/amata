use std::path::Path;

/// Write bytes atomically via temp-file + rename so crashes or concurrent
/// readers never observe a half-written document (previously every save
/// used `std::fs::write` directly onto the live file).
///
/// Note: `std::fs::rename` overwrites atomically on Unix; on Windows it
/// maps to MoveFileEx, which replaces the destination as well on modern
/// targets, but a fully hardened cross-platform story would want a
/// dedicated atomic-write crate or OS-specific handling.
pub fn atomic_write_bytes(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let tmp_path = path.with_extension(
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| format!("{ext}.tmp"))
            .unwrap_or_else(|| "tmp".to_string()),
    );

    {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }

    std::fs::rename(tmp_path, path)?;

    Ok(())
}

/// String variant of [`atomic_write_bytes`].
pub fn atomic_write_str(path: &Path, contents: &str) -> std::io::Result<()> {
    atomic_write_bytes(path, contents.as_bytes())
}
