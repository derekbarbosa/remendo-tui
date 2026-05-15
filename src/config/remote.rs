//! Remote Sashiko instance configuration.

use serde::Deserialize;

/// Default request timeout in seconds.
const fn default_timeout() -> u64 {
    15
}

/// Default maximum retry count.
const fn default_retries() -> u32 {
    3
}

/// Configuration for a single remote Sashiko instance.
///
/// Each remote represents an independent Sashiko deployment
/// that the TUI can connect to as a separate "mailbox."
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RemoteConfig {
    /// Human-readable name for this remote (used in the sidebar).
    pub name: String,
    /// Base URL of the Sashiko instance (e.g., `"https://sashiko.dev"`).
    pub url: String,
    /// Environment variable name containing an auth token (optional).
    /// The env var **name** is stored, not the token itself.
    #[serde(default)]
    pub auth_env: Option<String>,
    /// Request timeout in seconds. Defaults to 15.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Maximum number of retries on transient failures. Defaults to 3.
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_remote_with_defaults() {
        let toml_str = r#"
            name = "upstream"
            url = "https://sashiko.dev"
        "#;
        let remote: RemoteConfig = toml::from_str(toml_str).expect("parse remote");
        assert_eq!(remote.name, "upstream");
        assert_eq!(remote.url, "https://sashiko.dev");
        assert_eq!(remote.timeout_seconds, 15);
        assert_eq!(remote.max_retries, 3);
        assert!(remote.auth_env.is_none());
    }

    #[test]
    fn deserialize_remote_with_overrides() {
        let toml_str = r#"
            name = "staging"
            url = "https://staging.sashiko.dev"
            auth_env = "SASHIKO_TOKEN"
            timeout_seconds = 30
            max_retries = 5
        "#;
        let remote: RemoteConfig = toml::from_str(toml_str).expect("parse remote overrides");
        assert_eq!(remote.timeout_seconds, 30);
        assert_eq!(remote.max_retries, 5);
        assert_eq!(remote.auth_env.as_deref(), Some("SASHIKO_TOKEN"));
    }
}
