//! API client error types.

use std::fmt;
use std::time::Duration;

/// Errors from Sashiko API operations.
#[derive(Debug)]
pub enum ApiError {
    /// Network-level failure (DNS, connection, TLS, etc.).
    Network {
        /// The underlying error.
        source: Box<dyn std::error::Error + Send + Sync>,
        /// Name of the remote that failed.
        remote: String,
    },
    /// Server returned a non-2xx HTTP status.
    HttpStatus {
        /// HTTP status code.
        status: u16,
        /// Response body, if available.
        body: Option<String>,
        /// Name of the remote.
        remote: String,
    },
    /// Failed to parse the JSON response body.
    Deserialization {
        /// Description of the parse error.
        message: String,
        /// The endpoint that produced the bad response.
        endpoint: String,
    },
    /// The request exceeded the configured timeout.
    Timeout {
        /// The endpoint that timed out.
        endpoint: String,
        /// The timeout duration.
        duration: Duration,
    },
    /// Invalid client configuration (bad URL, etc.).
    Configuration(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network { source, remote } => {
                write!(f, "network error (remote '{remote}'): {source}")
            }
            Self::HttpStatus {
                status,
                body,
                remote,
            } => {
                write!(f, "HTTP {status} from remote '{remote}'")?;
                if let Some(b) = body {
                    let truncated = if b.len() > 200 {
                        &b[..b.floor_char_boundary(200)]
                    } else {
                        b
                    };
                    write!(f, ": {truncated}")?;
                }
                Ok(())
            }
            Self::Deserialization { message, endpoint } => {
                write!(f, "JSON parse error on {endpoint}: {message}")
            }
            Self::Timeout { endpoint, duration } => {
                write!(f, "request to {endpoint} timed out after {duration:?}")
            }
            Self::Configuration(msg) => write!(f, "client configuration error: {msg}"),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Network { source, .. } => Some(source.as_ref()),
            Self::Deserialization { .. }
            | Self::HttpStatus { .. }
            | Self::Timeout { .. }
            | Self::Configuration(_) => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn network_error_display() {
        let err = ApiError::Network {
            source: "connection refused".into(),
            remote: "upstream".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("upstream"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn http_status_display() {
        let err = ApiError::HttpStatus {
            status: 500,
            body: Some("internal server error".to_string()),
            remote: "staging".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("500"));
        assert!(msg.contains("staging"));
    }

    #[test]
    fn configuration_error_display() {
        let err = ApiError::Configuration("bad URL".to_string());
        assert!(err.to_string().contains("bad URL"));
    }

    #[test]
    fn timeout_error_display() {
        let err = ApiError::Timeout {
            endpoint: "/api/stats".to_string(),
            duration: Duration::from_secs(5),
        };
        let msg = err.to_string();
        assert!(msg.contains("/api/stats"));
        assert!(msg.contains("timed out"));
    }
}
