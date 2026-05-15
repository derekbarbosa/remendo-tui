//! Patch domain type — an individual diff within a patchset.

use serde::{Deserialize, Deserializer};
use std::fmt;

/// Status of an individual patch within a patchset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PatchStatus {
    /// Awaiting review.
    #[default]
    Pending,
    /// Review completed.
    Reviewed,
    /// Review failed.
    Failed,
    /// Patch could not be applied to the baseline.
    ApplyError,
    /// Unrecognized status from the API.
    Unknown,
}

impl fmt::Display for PatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Reviewed => write!(f, "Reviewed"),
            Self::Failed => write!(f, "Failed"),
            Self::ApplyError => write!(f, "Apply Error"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl<'de> Deserialize<'de> for PatchStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Option::<String>::deserialize(deserializer)?;
        Ok(match s.as_deref().map(str::to_lowercase).as_deref() {
            Some("reviewed") => Self::Reviewed,
            Some("failed") => Self::Failed,
            Some("apply error" | "applyerror" | "failedtoapply") => Self::ApplyError,
            Some("pending") | None => Self::Pending,
            Some(other) => {
                tracing::warn!(status = other, "unknown patch status, using Unknown");
                Self::Unknown
            }
        })
    }
}

/// An individual patch (diff) within a patchset.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Patch {
    /// Database identifier.
    #[serde(default)]
    pub id: i64,
    /// RFC 2822 `Message-ID` of the patch email.
    #[serde(default)]
    pub message_id: Option<String>,
    /// Position in the series (e.g., 1 for `[PATCH 1/5]`).
    #[serde(default)]
    pub part_index: Option<u32>,
    /// Subject line.
    #[serde(default)]
    pub subject: Option<String>,
    /// Review status of this individual patch.
    #[serde(default)]
    pub status: Option<PatchStatus>,
    /// Error message if the patch could not be applied.
    #[serde(default)]
    pub apply_error: Option<String>,
    /// Database ID of the corresponding message record.
    #[serde(default)]
    pub msg_db_id: Option<i64>,
}

#[cfg(test)]
    #[allow(clippy::expect_used)]
    mod tests {
    use super::*;

    #[test]
    fn patch_status_default() {
        assert_eq!(PatchStatus::default(), PatchStatus::Pending);
    }

    #[test]
    fn patch_status_display() {
        assert_eq!(PatchStatus::ApplyError.to_string(), "Apply Error");
        assert_eq!(PatchStatus::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn deserialize_patch() {
        let json = r#"{"id": 42, "part_index": 3, "subject": "fix something"}"#;
        let patch: Patch = serde_json::from_str(json).expect("deserialize patch");
        assert_eq!(patch.id, 42);
        assert_eq!(patch.part_index, Some(3));
    }

    #[test]
    fn deserialize_patch_unknown_status() {
        let json = r#"{"id": 1, "status": "NewThing"}"#;
        let patch: Patch = serde_json::from_str(json).expect("deserialize patch unknown status");
        assert_eq!(patch.status, Some(PatchStatus::Unknown));
    }
}
