//! Persistent bookmark storage for patchsets.
//!
//! Bookmarks are keyed by `(remote_name, patchset_id)` and persisted
//! as JSON to `$XDG_STATE_HOME/remendo/bookmarks.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// A single bookmark entry for serialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct BookmarkEntry {
    remote: String,
    patchset_id: i64,
}

/// On-disk bookmark file format.
#[derive(Debug, Serialize, Deserialize)]
struct BookmarkFile {
    version: u32,
    bookmarks: Vec<BookmarkEntry>,
}

/// In-memory bookmark store.
///
/// Holds the set of bookmarked patchsets and provides load/save
/// for JSON persistence.
#[derive(Debug, Clone, Default)]
pub struct BookmarkStore {
    bookmarks: HashSet<(String, i64)>,
}

impl BookmarkStore {
    /// Create an empty bookmark store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load bookmarks from a JSON file. Returns an empty store on any error.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::new();
        };
        let file: BookmarkFile = match serde_json::from_str(&content) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(error = %e, "failed to parse bookmarks file, starting empty");
                return Self::new();
            }
        };
        let bookmarks = file
            .bookmarks
            .into_iter()
            .map(|e| (e.remote, e.patchset_id))
            .collect();
        Self { bookmarks }
    }

    /// Save bookmarks to a JSON file. Uses atomic write (tmp + rename).
    ///
    /// # Errors
    ///
    /// Returns an error string if the write fails.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        use std::io::Write;

        let entries: Vec<BookmarkEntry> = self
            .bookmarks
            .iter()
            .map(|(remote, id)| BookmarkEntry {
                remote: remote.clone(),
                patchset_id: *id,
            })
            .collect();

        let file = BookmarkFile {
            version: 1,
            bookmarks: entries,
        };

        let json =
            serde_json::to_string_pretty(&file).map_err(|e| format!("serialize error: {e}"))?;

        // Atomic write: write to .tmp, then rename
        let tmp_path = path.with_extension("json.tmp");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut f =
            std::fs::File::create(&tmp_path).map_err(|e| format!("create tmp file: {e}"))?;
        f.write_all(json.as_bytes())
            .map_err(|e| format!("write tmp file: {e}"))?;
        std::fs::rename(&tmp_path, path).map_err(|e| format!("rename tmp to final: {e}"))?;
        Ok(())
    }

    /// Toggle a bookmark. Returns `true` if the patchset is now bookmarked.
    pub fn toggle(&mut self, remote: &str, patchset_id: i64) -> bool {
        let key = (remote.to_string(), patchset_id);
        if self.bookmarks.contains(&key) {
            self.bookmarks.remove(&key);
            false
        } else {
            self.bookmarks.insert(key);
            true
        }
    }

    /// Number of bookmarked patchsets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    /// Whether the bookmark store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }

    /// Check if a patchset is bookmarked.
    #[must_use]
    pub fn contains(&self, remote: &str, patchset_id: i64) -> bool {
        self.bookmarks.contains(&(remote.to_string(), patchset_id))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn toggle_adds_and_removes() {
        let mut store = BookmarkStore::new();
        assert!(store.toggle("upstream", 42));
        assert!(store.contains("upstream", 42));
        assert!(!store.toggle("upstream", 42));
        assert!(!store.contains("upstream", 42));
    }

    #[test]
    fn different_remotes_are_independent() {
        let mut store = BookmarkStore::new();
        store.toggle("upstream", 42);
        assert!(!store.contains("staging", 42));
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let store = BookmarkStore::load(Path::new("/nonexistent/bookmarks.json"));
        assert!(!store.contains("any", 1));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bookmarks.json");

        let mut store = BookmarkStore::new();
        store.toggle("upstream", 100);
        store.toggle("staging", 200);
        store.save(&path).expect("save");

        let loaded = BookmarkStore::load(&path);
        assert!(loaded.contains("upstream", 100));
        assert!(loaded.contains("staging", 200));
        assert!(!loaded.contains("upstream", 200));
    }
}
