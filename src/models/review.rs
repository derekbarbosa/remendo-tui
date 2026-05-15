//! Review domain type — an AI review of a single patch.

use serde::{Deserialize, Deserializer};
use std::fmt;

/// Status of an AI review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReviewStatus {
    /// Queued for review.
    #[default]
    Pending,
    /// Currently being reviewed.
    InReview,
    /// Review completed successfully.
    Reviewed,
    /// Review failed.
    Failed,
    /// Review was cancelled.
    Cancelled,
    /// Review was skipped.
    Skipped,
    /// Unrecognized status from the API.
    Unknown,
}

impl fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::InReview => write!(f, "In Review"),
            Self::Reviewed => write!(f, "Reviewed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Skipped => write!(f, "Skipped"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl<'de> Deserialize<'de> for ReviewStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Option::<String>::deserialize(deserializer)?;
        Ok(match s.as_deref().map(str::to_lowercase).as_deref() {
            Some("in review") => Self::InReview,
            Some("reviewed") => Self::Reviewed,
            Some("failed") => Self::Failed,
            Some("cancelled") => Self::Cancelled,
            Some("skipped") => Self::Skipped,
            Some("pending") | None => Self::Pending,
            Some(other) => {
                tracing::warn!(status = other, "unknown review status, using Unknown");
                Self::Unknown
            }
        })
    }
}

/// An AI review of a single patch.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Review {
    /// Database identifier.
    #[serde(default)]
    pub id: i64,
    /// ID of the patch being reviewed.
    #[serde(default)]
    pub patch_id: i64,
    /// Review status.
    #[serde(default)]
    pub status: ReviewStatus,
    /// LLM model used.
    #[serde(default)]
    pub model: Option<String>,
    /// LLM provider.
    #[serde(default)]
    pub provider: Option<String>,
    /// Inline review text (LKML-formatted reply).
    #[serde(default)]
    pub inline_review: Option<String>,
    /// Summary of the review.
    #[serde(default)]
    pub summary: Option<String>,
    /// Unix timestamp of review creation.
    #[serde(default)]
    pub created_at: Option<i64>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn review_status_default() {
        assert_eq!(ReviewStatus::default(), ReviewStatus::Pending);
    }

    #[test]
    fn review_status_display() {
        assert_eq!(ReviewStatus::InReview.to_string(), "In Review");
        assert_eq!(ReviewStatus::Cancelled.to_string(), "Cancelled");
    }

    #[test]
    fn deserialize_review() {
        let json = r#"{
            "id": 1,
            "patch_id": 42,
            "status": "Reviewed",
            "model": "gemini-3.1-pro-preview",
            "provider": "gemini",
            "summary": "Looks good"
        }"#;
        let review: Review = serde_json::from_str(json).expect("deserialize review");
        assert_eq!(review.status, ReviewStatus::Reviewed);
        assert_eq!(review.summary.as_deref(), Some("Looks good"));
    }

    #[test]
    fn deserialize_review_unknown_status() {
        let json = r#"{"id": 1, "patch_id": 1, "status": "FutureStatus"}"#;
        let review: Review = serde_json::from_str(json).expect("deserialize unknown review");
        assert_eq!(review.status, ReviewStatus::Unknown);
    }
}
