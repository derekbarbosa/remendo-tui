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
