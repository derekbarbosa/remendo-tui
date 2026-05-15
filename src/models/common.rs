//! Shared types used across multiple model modules.

use serde::Deserialize;
use std::fmt;

/// Severity level for a review finding.
///
/// Ordered from least to most severe: `Low < Medium < High < Critical`.
///
/// # Examples
///
/// ```
/// use remendo_tui::models::Severity;
///
/// let low = Severity::Low;
/// let critical = Severity::Critical;
/// assert!(low < critical);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Minor issue, informational.
    Low = 1,
    /// Moderate concern worth investigating.
    Medium = 2,
    /// Significant issue likely to cause problems.
    High = 3,
    /// Severe issue requiring immediate attention.
    Critical = 4,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// Git baseline reference for a patchset.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Baseline {
    /// Branch name (e.g., `"nf/HEAD"`).
    #[serde(default)]
    pub branch: Option<String>,
    /// Commit hash.
    #[serde(default)]
    pub commit: Option<String>,
    /// Repository URL.
    #[serde(default)]
    pub repo_url: Option<String>,
}

/// A tracked mailing list from the Sashiko instance.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MailingList {
    /// Human-readable name of the mailing list.
    pub name: String,
    /// NNTP group identifier.
    #[serde(default)]
    pub group: Option<String>,
}

/// Server health and queue statistics from `/api/stats`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ServerStats {
    /// Server status (typically `"ok"`).
    pub status: String,
    /// Sashiko version string.
    pub version: String,
    /// Number of patchsets pending review.
    #[serde(default)]
    pub pending: u64,
    /// Number of patchsets currently being reviewed.
    #[serde(default)]
    pub reviewing: u64,
    /// Total ingested messages.
    #[serde(default)]
    pub messages: u64,
    /// Total ingested patchsets.
    #[serde(default)]
    pub patchsets: u64,
}

/// Identifies a patchset, patch, or message by numeric ID or RFC 2822 Message-ID.
///
/// The Sashiko API accepts both forms for lookup endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatchId {
    /// Database numeric identifier.
    Numeric(i64),
    /// RFC 2822 `Message-ID` string.
    MessageId(String),
}

impl fmt::Display for PatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(id) => write!(f, "{id}"),
            Self::MessageId(mid) => write!(f, "{mid}"),
        }
    }
}

#[cfg(test)]
    #[allow(clippy::expect_used)]
    mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        let mut severities = vec![
            Severity::Critical,
            Severity::Low,
            Severity::High,
            Severity::Medium,
        ];
        severities.sort();
        assert_eq!(
            severities,
            vec![
                Severity::Low,
                Severity::Medium,
                Severity::High,
                Severity::Critical,
            ]
        );
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Low.to_string(), "Low");
        assert_eq!(Severity::Critical.to_string(), "Critical");
    }

    #[test]
    fn patch_id_display() {
        let numeric = PatchId::Numeric(42);
        assert_eq!(numeric.to_string(), "42");

        let msg = PatchId::MessageId("20260513-foo@example.com".to_string());
        assert_eq!(msg.to_string(), "20260513-foo@example.com");
    }
}
