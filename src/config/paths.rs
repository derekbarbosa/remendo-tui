//! XDG-compliant path resolution for application directories.

use super::error::ConfigError;
use std::path::PathBuf;

/// Resolved filesystem paths for the application.
///
/// Uses the XDG Base Directory Specification via the `dirs` crate,
/// with fallbacks to the current working directory when XDG paths
/// are unavailable.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Path to the configuration file.
    pub config_file: PathBuf,
    /// Directory for cached API responses.
    pub cache_dir: PathBuf,
    /// Directory for persistent state (bookmarks, read-state).
    pub state_dir: PathBuf,
}

impl AppPaths {
    /// Resolve all application paths using the XDG spec.
    ///
    /// Falls back to CWD-relative paths if the user's home directory
    /// cannot be determined.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Io`] if the current directory cannot be
    /// determined during fallback resolution.
    pub fn resolve() -> Result<Self, ConfigError> {
        let config_file = dirs::config_dir().map_or_else(
            || PathBuf::from("config.toml"),
            |d| d.join("remendo").join("config.toml"),
        );

        let cache_dir = dirs::cache_dir().map_or_else(
            || PathBuf::from(".cache").join("remendo"),
            |d| d.join("remendo"),
        );

        let state_dir = dirs::state_dir().map_or_else(
            || PathBuf::from(".local").join("state").join("remendo"),
            |d| d.join("remendo"),
        );

        Ok(Self {
            config_file,
            cache_dir,
            state_dir,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn resolve_produces_paths() {
        let paths = AppPaths::resolve().expect("resolve paths");
        assert!(
            paths.config_file.to_string_lossy().ends_with("config.toml"),
            "config_file should end with config.toml, got: {:?}",
            paths.config_file
        );
    }
}
