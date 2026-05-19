//! Configuration error and warning types.

use std::fmt;

/// Errors that can occur during configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    /// Filesystem I/O error reading the config file.
    Io(std::io::Error),
    /// TOML parsing/deserialization error.
    Parse(toml::de::Error),
    /// Validation errors after successful parsing.
    Validation(Vec<String>),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "config I/O error: {e}"),
            Self::Parse(e) => write!(f, "config parse error: {e}"),
            Self::Validation(msgs) => {
                write!(f, "config validation errors: ")?;
                for (i, msg) in msgs.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{msg}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
            Self::Validation(_) => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

/// Non-fatal warnings collected during configuration loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWarning {
    /// No config file was found; using compiled-in defaults.
    NoFile,
    /// A remote URL could not be parsed.
    InvalidRemoteUrl {
        /// Name of the misconfigured remote.
        name: String,
        /// The invalid URL string.
        url: String,
    },
    /// A non-critical parse issue.
    ParseWarning(String),
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFile => write!(f, "no config file found, using defaults"),
            Self::InvalidRemoteUrl { name, url } => {
                write!(f, "remote '{name}' has invalid URL: {url}")
            }
            Self::ParseWarning(msg) => write!(f, "config warning: {msg}"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn config_error_display_io() {
        let err = ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not found",
        ));
        let s = err.to_string();
        assert!(s.contains("config I/O error"), "got: {s}");
    }

    #[test]
    fn config_error_display_parse() {
        // Create a real TOML parse error
        let toml_err = toml::from_str::<toml::Value>("{{bad").unwrap_err();
        let err = ConfigError::Parse(toml_err);
        let s = err.to_string();
        assert!(s.contains("config parse error"), "got: {s}");
    }

    #[test]
    fn config_error_display_validation() {
        let err =
            ConfigError::Validation(vec!["missing url".to_string(), "bad timeout".to_string()]);
        let s = err.to_string();
        assert!(s.contains("config validation errors"), "got: {s}");
        assert!(s.contains("missing url"), "got: {s}");
        assert!(s.contains("; bad timeout"), "got: {s}");
    }

    #[test]
    fn config_error_display_validation_single() {
        let err = ConfigError::Validation(vec!["one error".to_string()]);
        let s = err.to_string();
        assert!(!s.contains(';'), "single error should not contain ';': {s}");
    }

    #[test]
    fn config_error_source_io() {
        let err = ConfigError::Io(std::io::Error::other("test"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn config_error_source_parse() {
        let toml_err = toml::from_str::<toml::Value>("{{bad").unwrap_err();
        let err = ConfigError::Parse(toml_err);
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn config_error_source_validation_is_none() {
        let err = ConfigError::Validation(vec![]);
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn config_warning_display_no_file() {
        let w = ConfigWarning::NoFile;
        assert_eq!(w.to_string(), "no config file found, using defaults");
    }

    #[test]
    fn config_warning_display_invalid_url() {
        let w = ConfigWarning::InvalidRemoteUrl {
            name: "prod".to_string(),
            url: "not-a-url".to_string(),
        };
        let s = w.to_string();
        assert!(s.contains("prod"), "got: {s}");
        assert!(s.contains("not-a-url"), "got: {s}");
    }

    #[test]
    fn config_warning_display_parse_warning() {
        let w = ConfigWarning::ParseWarning("something odd".to_string());
        assert!(w.to_string().contains("something odd"));
    }
}
