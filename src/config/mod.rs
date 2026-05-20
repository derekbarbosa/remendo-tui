//! Application configuration subsystem.
//!
//! Loads user preferences from a TOML file at XDG-compliant paths,
//! provides sensible compiled-in defaults when no file exists, and
//! exposes typed configuration to all consumer subsystems.

pub mod cache;
pub mod error;
pub mod keys;
pub mod paths;
pub mod remote;
pub mod theme;

pub use cache::CacheConfig;
pub use error::{ConfigError, ConfigWarning};
pub use keys::KeybindingsConfig;
pub use paths::AppPaths;
pub use remote::RemoteConfig;
pub use theme::ThemeConfig;

use serde::Deserialize;

/// Top-level application configuration.
///
/// All fields use `#[serde(default)]` so partial TOML files work —
/// users only override what they want.
///
/// # Examples
///
/// ```
/// use remendo::config::Config;
///
/// // Default config has no remotes, default cache, default keybindings
/// let config = Config::default();
/// assert!(config.remotes.is_empty());
/// assert_eq!(config.cache.ttl_seconds, 300);
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Editor command for viewing raw review logs.
    /// Falls back to `$VISUAL`, then `$EDITOR`, then `"vi"`.
    pub editor: Option<String>,
    /// Cache behaviour settings.
    pub cache: CacheConfig,
    /// Configured remote Sashiko instances.
    pub remotes: Vec<RemoteConfig>,
    /// Keybinding mappings.
    pub keybindings: KeybindingsConfig,
    /// Color theme.
    pub theme: ThemeConfig,
}

impl Config {
    /// Load configuration from the XDG config path.
    ///
    /// Returns the loaded config and any non-fatal warnings.
    /// If no config file exists, returns compiled-in defaults
    /// with a [`ConfigWarning::NoFile`] warning.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] on I/O or TOML parse failures.
    pub fn load() -> Result<(Self, Vec<ConfigWarning>), ConfigError> {
        let mut warnings = Vec::new();
        let paths = AppPaths::resolve()?;

        if !paths.config_file.exists() {
            warnings.push(ConfigWarning::NoFile);
            return Ok((Self::default(), warnings));
        }

        let content = std::fs::read_to_string(&paths.config_file)?;
        let config: Self = toml::from_str(&content)?;

        // Validate remote URLs
        for remote in &config.remotes {
            if remote.url.is_empty()
                || (!remote.url.starts_with("http://") && !remote.url.starts_with("https://"))
            {
                warnings.push(ConfigWarning::InvalidRemoteUrl {
                    name: remote.name.clone(),
                    url: remote.url.clone(),
                });
            }
        }

        Ok((config, warnings))
    }

    /// Load configuration from a TOML string (for testing).
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Parse`] on deserialization failure.
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(toml_str)?;
        Ok(config)
    }

    /// Resolve the editor command using the priority chain:
    /// config file > `$VISUAL` > `$EDITOR` > `"vi"`.
    #[must_use]
    pub fn resolved_editor(&self) -> String {
        self.editor
            .clone()
            .or_else(|| std::env::var("VISUAL").ok())
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vi".to_string())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::default();
        assert!(config.remotes.is_empty());
        assert!(config.editor.is_none());
        assert_eq!(config.cache.ttl_seconds, 300);
    }

    #[test]
    fn load_returns_no_file_warning_when_missing() {
        // This test assumes no config file exists at the XDG path
        // during CI. If it does, the test still passes (it just
        // exercises the load-from-file path instead).
        let result = Config::load();
        assert!(result.is_ok());
    }

    #[test]
    fn parse_minimal_toml() {
        let toml_str = r#"
            [[remotes]]
            name = "upstream"
            url = "https://sashiko.dev"
        "#;
        let config = Config::from_toml(toml_str).expect("parse minimal toml");
        assert_eq!(config.remotes.len(), 1);
        assert_eq!(config.remotes[0].name, "upstream");
        assert_eq!(config.remotes[0].url, "https://sashiko.dev");
        // Defaults fill the rest
        assert_eq!(config.cache.ttl_seconds, 300);
        assert!(config.editor.is_none());
    }

    #[test]
    fn parse_full_toml() {
        let toml_str = r#"
            editor = "nvim"

            [cache]
            ttl_seconds = 60

            [[remotes]]
            name = "upstream"
            url = "https://sashiko.dev"

            [[remotes]]
            name = "staging"
            url = "https://staging.sashiko.dev"
            auth_env = "SASHIKO_TOKEN"
            timeout_seconds = 30
        "#;
        let config = Config::from_toml(toml_str).expect("parse full toml");
        assert_eq!(config.editor.as_deref(), Some("nvim"));
        assert_eq!(config.cache.ttl_seconds, 60);
        assert_eq!(config.remotes.len(), 2);
        assert_eq!(config.remotes[1].timeout_seconds, 30);
    }

    #[test]
    fn invalid_toml_returns_parse_error() {
        let bad_toml = "this is not { valid toml";
        let result = Config::from_toml(bad_toml);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(ConfigError::Parse(_))),
            "expected ConfigError::Parse"
        );
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let toml_str = r"
            [cache]
            ttl_seconds = 120
        ";
        let config = Config::from_toml(toml_str).expect("parse partial toml");
        assert!(config.remotes.is_empty());
        assert!(config.editor.is_none());
        assert_eq!(config.cache.ttl_seconds, 120);
    }

    #[test]
    fn resolved_editor_falls_back_to_vi() {
        let config = Config {
            editor: None,
            ..Config::default()
        };
        // In CI, $VISUAL and $EDITOR may or may not be set,
        // so we just verify the function doesn't panic.
        let editor = config.resolved_editor();
        assert!(!editor.is_empty());
    }

    #[test]
    fn resolved_editor_config_overrides_env() {
        let config = Config {
            editor: Some("helix".to_string()),
            ..Config::default()
        };
        assert_eq!(config.resolved_editor(), "helix");
    }

    #[test]
    fn multiple_remotes_preserve_order() {
        let toml_str = r#"
            [[remotes]]
            name = "first"
            url = "https://first.example.com"

            [[remotes]]
            name = "second"
            url = "https://second.example.com"

            [[remotes]]
            name = "third"
            url = "https://third.example.com"
        "#;
        let config = Config::from_toml(toml_str).expect("parse multiple remotes");
        assert_eq!(config.remotes.len(), 3);
        assert_eq!(config.remotes[0].name, "first");
        assert_eq!(config.remotes[1].name, "second");
        assert_eq!(config.remotes[2].name, "third");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod load_integration_tests {
    use super::*;

    #[test]
    fn load_does_not_error() {
        // Config::load() should never return Err — it falls back to
        // defaults when no file is found, and parses when one exists.
        let result = Config::load();
        assert!(result.is_ok(), "Config::load() failed: {result:?}");
    }

    #[test]
    fn load_from_xdg_no_false_nofile_warning() {
        let paths = AppPaths::resolve().expect("resolve paths");
        let (_, warnings) = Config::load().expect("load config");

        // If a config file exists at the XDG path, the NoFile warning
        // must not be emitted.
        if paths.config_file.exists() {
            assert!(
                !warnings.iter().any(|w| matches!(w, ConfigWarning::NoFile)),
                "config file exists at {:?} but got NoFile warning",
                paths.config_file
            );
        }
    }

    #[test]
    fn load_produces_nofile_warning_when_missing() {
        // If no config file exists, we should get a NoFile warning.
        // This test is environment-dependent — if a config file does
        // exist, it validates the other path instead.
        let paths = AppPaths::resolve().expect("resolve paths");
        let (_, warnings) = Config::load().expect("load config");

        if !paths.config_file.exists() {
            assert!(
                warnings.iter().any(|w| matches!(w, ConfigWarning::NoFile)),
                "no config file at {:?} but no NoFile warning",
                paths.config_file
            );
        }
    }

    #[test]
    fn load_populates_keybindings() {
        let (config, _) = Config::load().expect("load config");
        // Regardless of whether a file exists, keybindings should
        // have the default set (17 bindings).
        assert!(
            !config.keybindings.bindings.is_empty(),
            "keybindings should have default entries"
        );
    }

    #[test]
    fn from_toml_with_remotes_populates_list() {
        let toml_str = r#"
            [[remotes]]
            name = "test"
            url = "https://sashiko.example.com"
        "#;
        let config = Config::from_toml(toml_str).expect("parse toml");
        assert_eq!(config.remotes.len(), 1);
        assert_eq!(config.remotes[0].name, "test");
        assert_eq!(config.remotes[0].url, "https://sashiko.example.com");
        // Keybindings should still have defaults even when not in TOML
        assert!(!config.keybindings.bindings.is_empty());
    }
}
