use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// Robust Watcher to detect external file modifications with content hashing,
/// debouncing (for in-flight atomic writes), same-size change detection,
/// and self-write loop suppression.
#[derive(Debug, Clone)]
pub struct FileWatcher {
    pub file_path: PathBuf,
    pub last_modified: Option<SystemTime>,
    pub last_file_size: u64,
    pub last_content_hash: u64,
    pub has_external_change: bool,
    pub last_check_time: Instant,
    pub debounce_duration: Duration,
    pub pending_invalid_last_change: Option<Instant>,
    pub pending_invalid_last_hash: u64,
    /// Content hash of the last invalid SVG we already warned about, so a
    /// persistently broken external file notifies once instead of spamming
    /// every frame.
    pub warned_invalid_hash: u64,
    pub partial_write_timeout: Duration,
    pub last_full_verify: Instant,
    pub periodic_verify_interval: Duration,
}

pub(crate) fn compute_hash(data: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

impl FileWatcher {
    pub fn new(path: PathBuf) -> Self {
        let mut watcher = Self {
            file_path: path,
            last_modified: None,
            last_file_size: 0,
            last_content_hash: 0,
            has_external_change: false,
            last_check_time: Instant::now(),
            debounce_duration: Duration::from_millis(50),
            pending_invalid_last_change: None,
            pending_invalid_last_hash: 0,
            warned_invalid_hash: 0,
            partial_write_timeout: Duration::from_millis(500),
            last_full_verify: Instant::now(),
            periodic_verify_interval: Duration::from_millis(1000),
        };
        watcher.update_timestamp();
        watcher
    }

    pub fn set_partial_write_timeout(&mut self, timeout: Duration) {
        self.partial_write_timeout = timeout;
    }

    pub fn set_periodic_verify_interval(&mut self, interval: Duration) {
        self.periodic_verify_interval = interval;
    }

    /// Returns the adaptive periodic verification interval based on file size.
    /// Small files (<1MB) verify every 1.0s, medium files (1MB-10MB) every 3.0s,
    /// large files (10MB-30MB) every 5.0s, and very large files (>=30MB) every 10.0s.
    /// If an explicit custom interval was set (different from default 1000ms), it is respected.
    pub fn effective_verify_interval(&self) -> Duration {
        if self.periodic_verify_interval != Duration::from_millis(1000) {
            return self.periodic_verify_interval;
        }

        if self.last_file_size < 1024 * 1024 {
            Duration::from_millis(1000)
        } else if self.last_file_size < 10 * 1024 * 1024 {
            Duration::from_millis(3000)
        } else if self.last_file_size < 30 * 1024 * 1024 {
            Duration::from_millis(5000)
        } else {
            Duration::from_millis(10000)
        }
    }

    /// Mark that Amata itself wrote this content to disk, preventing self-write loop
    pub fn mark_saved(&mut self, content: &str) {
        self.warned_invalid_hash = 0;
        self.last_content_hash = compute_hash(content);
        self.last_file_size = content.len() as u64;
        if let Ok(metadata) = std::fs::metadata(&self.file_path) {
            self.last_modified = metadata.modified().ok();
        }
        self.has_external_change = false;
        self.pending_invalid_last_change = None;
        self.pending_invalid_last_hash = 0;
        self.last_full_verify = Instant::now();
    }

    /// Refresh known mtime, size, and content hash from disk
    pub fn update_timestamp(&mut self) {
        if let Ok(metadata) = std::fs::metadata(&self.file_path) {
            self.last_modified = metadata.modified().ok();
            self.last_file_size = metadata.len();
            if let Ok(content) = std::fs::read_to_string(&self.file_path) {
                self.last_content_hash = compute_hash(&content);
            }
            self.has_external_change = false;
            self.pending_invalid_last_change = None;
            self.pending_invalid_last_hash = 0;
            self.last_full_verify = Instant::now();
        }
    }

    /// Check if the file on disk was modified externally since last check.
    /// Returns Some(valid_content) if genuine, complete external change occurred.
    pub fn check_for_changes(&mut self) -> bool {
        self.check_for_external_content().is_some()
    }

    /// Inspect file on disk and return Some(content) if an external change is verified.
    /// Handles in-flight partial writes and verifies content hash differs from last known state.
    /// If content remains incomplete/corrupted after partial_write_timeout (idle since last change),
    /// yields it so error is handled.
    pub fn check_for_external_content(&mut self) -> Option<String> {
        let metadata = match std::fs::metadata(&self.file_path) {
            Ok(m) => m,
            Err(_) => return None,
        };

        let current_mod = metadata.modified().ok();
        let current_size = metadata.len();

        // Check if metadata hints at a potential modification
        let metadata_changed = match (self.last_modified, current_mod) {
            (Some(last_m), Some(curr_m)) => curr_m > last_m || current_size != self.last_file_size,
            (None, Some(_)) => true,
            _ => current_size != self.last_file_size,
        };

        let interval = self.effective_verify_interval();
        let is_periodic_check =
            interval > Duration::ZERO && self.last_full_verify.elapsed() >= interval;

        // Read content and check content hash if metadata changed OR periodic verification fires
        if metadata_changed
            || current_size != self.last_file_size
            || self.last_content_hash == 0
            || is_periodic_check
        {
            self.last_full_verify = Instant::now();

            if let Ok(content) = std::fs::read_to_string(&self.file_path) {
                let trimmed = content.trim();
                let current_content_hash = compute_hash(&content);

                if trimmed.is_empty() {
                    // Empty / zero-byte file - check idle timeout
                    if let Some(since) = self.pending_invalid_last_change {
                        if current_content_hash != self.pending_invalid_last_hash {
                            self.pending_invalid_last_change = Some(Instant::now());
                            self.pending_invalid_last_hash = current_content_hash;
                            return None;
                        } else if since.elapsed() < self.partial_write_timeout {
                            return None;
                        }
                    } else {
                        self.pending_invalid_last_change = Some(Instant::now());
                        self.pending_invalid_last_hash = current_content_hash;
                        return None;
                    }
                }

                // Complete SVG: either has closing </svg> or root <svg ... /> itself is self-closing
                let is_complete_svg = trimmed.contains("</svg>")
                    || (trimmed.starts_with("<svg")
                        && trimmed.ends_with("/>")
                        && !trimmed[4..trimmed.len().saturating_sub(2)].contains('>'));
                if !is_complete_svg {
                    // Incomplete write: calculate idle timeout relative to when content last changed
                    if let Some(since) = self.pending_invalid_last_change {
                        if current_content_hash != self.pending_invalid_last_hash {
                            // File was actively modified since last tick: reset idle timeout
                            self.pending_invalid_last_change = Some(Instant::now());
                            self.pending_invalid_last_hash = current_content_hash;
                            return None;
                        } else if since.elapsed() < self.partial_write_timeout {
                            // Stable, but still within idle timeout window
                            return None;
                        }
                        // Timeout expired: file has been idle and incomplete for partial_write_timeout
                    } else {
                        self.pending_invalid_last_change = Some(Instant::now());
                        self.pending_invalid_last_hash = current_content_hash;
                        return None; // first observation of incomplete write, defer
                    }
                } else {
                    self.pending_invalid_last_change = None;
                    self.pending_invalid_last_hash = 0;
                }

                if current_content_hash != self.last_content_hash {
                    self.last_modified = current_mod;
                    self.last_file_size = current_size;
                    self.has_external_change = true;
                    return Some(content);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_verify_interval() {
        let mut watcher = FileWatcher::new(PathBuf::from("dummy.svg"));
        assert_eq!(
            watcher.effective_verify_interval(),
            Duration::from_millis(1000)
        );

        // Medium file: 5MB
        watcher.last_file_size = 5 * 1024 * 1024;
        assert_eq!(
            watcher.effective_verify_interval(),
            Duration::from_millis(3000)
        );

        // Large file: 20MB
        watcher.last_file_size = 20 * 1024 * 1024;
        assert_eq!(
            watcher.effective_verify_interval(),
            Duration::from_millis(5000)
        );

        // Very large file: 50MB
        watcher.last_file_size = 50 * 1024 * 1024;
        assert_eq!(
            watcher.effective_verify_interval(),
            Duration::from_millis(10000)
        );

        // Explicit override respects custom setting
        watcher.set_periodic_verify_interval(Duration::from_millis(250));
        assert_eq!(
            watcher.effective_verify_interval(),
            Duration::from_millis(250)
        );
    }
}
