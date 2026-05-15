//! Cache behaviour configuration.

use serde::Deserialize;
use std::path::PathBuf;

/// Default cache TTL in seconds.
const fn default_ttl() -> u64 {
    300
}

/// Settings controlling local API response caching.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Override for the cache directory (default: `$XDG_CACHE_HOME/remendo/`).
    pub dir: Option<PathBuf>,
    /// Time-to-live for cached responses, in seconds.
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: None,
            ttl_seconds: default_ttl(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_cache_config() {
        let cfg = CacheConfig::default();
        assert!(cfg.dir.is_none());
        assert_eq!(cfg.ttl_seconds, 300);
    }

    #[test]
    fn deserialize_cache_override() {
        let toml_str = r#"
            dir = "/tmp/remendo-cache"
            ttl_seconds = 60
        "#;
        let cfg: CacheConfig = toml::from_str(toml_str).expect("parse cache config");
        assert_eq!(cfg.dir.as_deref(), Some(std::path::Path::new("/tmp/remendo-cache")));
        assert_eq!(cfg.ttl_seconds, 60);
    }
}
