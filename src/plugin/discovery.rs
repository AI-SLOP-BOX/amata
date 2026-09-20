//! Headless plugin discovery.
//!
//! The GUI keeps plugins in memory (`PluginManager`), but the CLI had no way of
//! finding them at all (`amata plugins --info <id>` used to print a
//! "not yet implemented" placeholder). Amata plugins are Rhai scripts, so
//! discovery is a directory scan plus metadata parsing.
//!
//! Metadata lives in the leading comment block of the script:
//!
//! ```rhai
//! // @name: Poster Grid
//! // @id: poster-grid
//! // @version: 1.2.0
//! // @author: Amata Contributors
//! // @description: Repeats the selection on a grid
//! #{ width: 1000.0, objects: [] }
//! ```
//!
//! Search order: `$AMATA_PLUGIN_DIR` / `$IRASU_PLUGIN_DIR` (path-list
//! separated), `./plugins`, then `~/.irasu/plugins`.

use std::path::{Path, PathBuf};

/// Scripts larger than this are ignored (a plugin is a script, not an asset).
pub const MAX_PLUGIN_BYTES: u64 = 8 * 1024 * 1024;

/// Bytes of the script kept for `--info` previews.
const PREVIEW_BYTES: usize = 8 * 1024;

/// Number of leading comment lines scanned for `@key: value` metadata.
const METADATA_SCAN_LINES: usize = 40;

/// Metadata declared in the script header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

/// A plugin script found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub line_count: usize,
    /// Leading bytes of the script, used for `--info` previews.
    pub head: String,
}

impl DiscoveredPlugin {
    /// First `max_lines` lines of the script (for terminal previews).
    pub fn preview(&self, max_lines: usize) -> String {
        self.head
            .lines()
            .take(max_lines)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Parse `// @key: value` metadata from the leading comment block.
/// Falls back to `fallback_id` (the file stem) and the id for the name.
pub fn parse_metadata(source: &str, fallback_id: &str) -> PluginMetadata {
    let mut meta = PluginMetadata {
        id: fallback_id.to_string(),
        name: String::new(),
        version: String::new(),
        author: String::new(),
        description: String::new(),
    };

    for line in source.lines().take(METADATA_SCAN_LINES) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(comment) = trimmed.strip_prefix("//") else {
            // First non-comment line ends the header block.
            break;
        };
        let comment = comment.trim();
        // Accept both `// @key: value` and `// * @key: value` style headers.
        let comment = comment.strip_prefix('*').map(str::trim).unwrap_or(comment);
        let Some(body) = comment.strip_prefix('@') else {
            continue;
        };
        let Some((key, value)) = body.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        match key.trim().to_ascii_lowercase().as_str() {
            "id" | "slug" => meta.id = value,
            "name" | "title" => meta.name = value,
            "version" => meta.version = value,
            "author" | "by" => meta.author = value,
            "description" | "desc" | "about" => meta.description = value,
            _ => {}
        }
    }

    if meta.name.is_empty() {
        meta.name = meta.id.clone();
    }
    if meta.version.is_empty() {
        meta.version = "0.0.0".to_string();
    }
    if meta.author.is_empty() {
        meta.author = "unknown".to_string();
    }
    meta
}

/// Directories searched for `*.rhai` plugins, in priority order.
pub fn default_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    let sep = if cfg!(windows) { ';' } else { ':' };
    for var in ["AMATA_PLUGIN_DIR", "IRASU_PLUGIN_DIR"] {
        if let Ok(value) = std::env::var(var) {
            for part in value.split(sep) {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let path = PathBuf::from(part);
                if !dirs.contains(&path) {
                    dirs.push(path);
                }
            }
        }
    }

    dirs.push(PathBuf::from("plugins"));

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();
    if let Some(home) = home {
        let path = Path::new(&home).join(".irasu").join("plugins");
        if !dirs.contains(&path) {
            dirs.push(path);
        }
    }

    dirs
}

/// Scan `dirs` for plugin scripts. Missing/unreadable directories are skipped,
/// oversized files are ignored, and duplicates (the same file reachable through
/// two search paths) are collapsed.
pub fn discover_in(dirs: &[PathBuf]) -> Vec<DiscoveredPlugin> {
    let mut found: Vec<DiscoveredPlugin> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let is_rhai = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("rhai"))
                .unwrap_or(false);
            if !is_rhai {
                continue;
            }
            let unique = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if seen.contains(&unique) {
                continue;
            }
            let Ok(fs_meta) = entry.metadata() else {
                continue;
            };
            if fs_meta.len() > MAX_PLUGIN_BYTES {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            seen.push(unique);

            let fallback_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("plugin")
                .trim()
                .to_string();
            let meta = parse_metadata(&source, &fallback_id);
            found.push(DiscoveredPlugin {
                id: meta.id,
                name: meta.name,
                version: meta.version,
                author: meta.author,
                description: meta.description,
                path,
                size_bytes: fs_meta.len(),
                line_count: source.lines().count(),
                head: source.chars().take(PREVIEW_BYTES).collect(),
            });
        }
    }

    found.sort_by_key(|plugin| plugin.id.to_lowercase());
    found
}

/// Find a plugin by declared id, display name, or file stem (case-insensitive).
pub fn find_plugin<'a>(plugins: &'a [DiscoveredPlugin], key: &str) -> Option<&'a DiscoveredPlugin> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    let matches = |candidate: &str| candidate.eq_ignore_ascii_case(key);

    plugins
        .iter()
        .find(|p| matches(&p.id))
        .or_else(|| plugins.iter().find(|p| matches(&p.name)))
        .or_else(|| {
            plugins.iter().find(|p| {
                p.path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(matches)
                    .unwrap_or(false)
            })
        })
}
