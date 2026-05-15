//! Patchset and patchset-detail domain types.

use super::common::{Baseline, Severity};
use super::message::ThreadMessage;
use super::patch::Patch;
use super::review::Review;
use serde::{Deserialize, Deserializer};
use std::fmt;

/// Lifecycle status of a patchset.
///
/// Deserializes case-insensitively from API strings. Unknown values
/// map to [`PatchsetStatus::Unknown`] with a warning log instead of
/// failing deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PatchsetStatus {
    /// Not all parts have been received yet.
    #[default]
    Incomplete,
    /// Queued for review.
    Pending,
    /// Currently being reviewed.
    InReview,
    /// Review completed successfully.
    Reviewed,
    /// Review failed.
    Failed,
    /// Patch could not be applied to the baseline.
    FailedToApply,
    /// Review was cancelled.
    Cancelled,
    /// Patchset was skipped.
    Skipped,
    /// Unrecognized status from the API.
    Unknown,
}

impl fmt::Display for PatchsetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete => write!(f, "Incomplete"),
            Self::Pending => write!(f, "Pending"),
            Self::InReview => write!(f, "In Review"),
            Self::Reviewed => write!(f, "Reviewed"),
            Self::Failed => write!(f, "Failed"),
            Self::FailedToApply => write!(f, "Failed to Apply"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Skipped => write!(f, "Skipped"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl<'de> Deserialize<'de> for PatchsetStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Option::<String>::deserialize(deserializer)?;
        Ok(match s.as_deref().map(str::to_lowercase).as_deref() {
            Some("pending") => Self::Pending,
            Some("in review") => Self::InReview,
            Some("reviewed") => Self::Reviewed,
            Some("failed") => Self::Failed,
            Some("failedtoapply" | "failed to apply") => Self::FailedToApply,
            Some("cancelled") => Self::Cancelled,
            Some("skipped") => Self::Skipped,
            Some("incomplete") | None => Self::Incomplete,
            Some(other) => {
                tracing::warn!(status = other, "unknown patchset status, using Unknown");
                Self::Unknown
            }
        })
    }
}

/// Aggregated finding severity counts for a patchset.
///
/// Collapses the four separate `findings_*` fields from the API
/// into a single struct with convenience methods.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FindingCounts {
    /// Count of low-severity findings.
    pub low: u32,
    /// Count of medium-severity findings.
    pub medium: u32,
    /// Count of high-severity findings.
    pub high: u32,
    /// Count of critical-severity findings.
    pub critical: u32,
}

impl FindingCounts {
    /// Total number of findings across all severities.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.low + self.medium + self.high + self.critical
    }

    /// Returns the highest severity level that has a non-zero count.
    #[must_use]
    pub fn max_severity(&self) -> Option<Severity> {
        if self.critical > 0 {
            Some(Severity::Critical)
        } else if self.high > 0 {
            Some(Severity::High)
        } else if self.medium > 0 {
            Some(Severity::Medium)
        } else if self.low > 0 {
            Some(Severity::Low)
        } else {
            None
        }
    }

    /// Whether all finding counts are zero.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// Helper for deserializing the flat `findings_*` API fields into
/// a nested `FindingCounts` struct on `Patchset`.
#[derive(Deserialize)]
struct PatchsetRaw {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    status: PatchsetStatus,
    #[serde(default)]
    thread_id: Option<i64>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    date: Option<i64>,
    #[serde(default)]
    message_id: Option<String>,
    #[serde(default)]
    total_parts: Option<u32>,
    #[serde(default)]
    received_parts: Option<u32>,
    #[serde(default)]
    subsystems: Vec<String>,
    #[serde(default)]
    findings_low: Option<i64>,
    #[serde(default)]
    findings_medium: Option<i64>,
    #[serde(default)]
    findings_high: Option<i64>,
    #[serde(default)]
    findings_critical: Option<i64>,
    #[serde(default)]
    baseline_id: Option<i64>,
    #[serde(default)]
    failed_reason: Option<String>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    provider: Option<String>,
}

/// A patchset summary, as returned by `GET /api/patchsets`.
///
/// This is the list-view representation. For full detail including
/// patches, reviews, and thread, see [`PatchsetDetail`].
#[derive(Debug, Clone, PartialEq)]
pub struct Patchset {
    /// Database identifier.
    pub id: i64,
    /// Subject line of the cover letter or first patch.
    pub subject: Option<String>,
    /// Current lifecycle status.
    pub status: PatchsetStatus,
    /// Thread identifier.
    pub thread_id: Option<i64>,
    /// Author email or name.
    pub author: Option<String>,
    /// Unix timestamp of the patchset date.
    pub date: Option<i64>,
    /// RFC 2822 `Message-ID`.
    pub message_id: Option<String>,
    /// Total number of patches expected in the series.
    pub total_parts: Option<u32>,
    /// Number of patches received so far.
    pub received_parts: Option<u32>,
    /// Subsystem tags (e.g., `["LKML", "netdev"]`).
    pub subsystems: Vec<String>,
    /// Aggregated finding severity counts.
    pub findings: FindingCounts,
    /// Baseline identifier used for review.
    pub baseline_id: Option<i64>,
    /// Reason for failure, if applicable.
    pub failed_reason: Option<String>,
    /// LLM model used for review.
    pub model_name: Option<String>,
    /// LLM provider.
    pub provider: Option<String>,
}

impl Patchset {
    /// Author name, or `"(unknown)"` if not provided.
    #[must_use]
    pub fn author(&self) -> &str {
        self.author.as_deref().unwrap_or("(unknown)")
    }

    /// Subject line, or `"(no subject)"` if not provided.
    #[must_use]
    pub fn subject(&self) -> &str {
        self.subject.as_deref().unwrap_or("(no subject)")
    }
}

impl<'de> Deserialize<'de> for Patchset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = PatchsetRaw::deserialize(deserializer)?;
        let clamp =
            |v: Option<i64>| -> u32 { u32::try_from(v.unwrap_or(0).max(0)).unwrap_or(u32::MAX) };
        Ok(Self {
            id: raw.id,
            subject: raw.subject,
            status: raw.status,
            thread_id: raw.thread_id,
            author: raw.author,
            date: raw.date,
            message_id: raw.message_id,
            total_parts: raw.total_parts,
            received_parts: raw.received_parts,
            subsystems: raw.subsystems,
            findings: FindingCounts {
                low: clamp(raw.findings_low),
                medium: clamp(raw.findings_medium),
                high: clamp(raw.findings_high),
                critical: clamp(raw.findings_critical),
            },
            baseline_id: raw.baseline_id,
            failed_reason: raw.failed_reason,
            model_name: raw.model_name,
            provider: raw.provider,
        })
    }
}

/// Full patchset detail, as returned by `GET /api/patch`.
///
/// Includes nested patches, reviews, thread messages, and baseline info.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PatchsetDetail {
    /// Database identifier.
    #[serde(default)]
    pub id: i64,
    /// Subject line.
    #[serde(default)]
    pub subject: Option<String>,
    /// Lifecycle status.
    #[serde(default)]
    pub status: PatchsetStatus,
    /// Author email or name.
    #[serde(default)]
    pub author: Option<String>,
    /// Unix timestamp.
    #[serde(default)]
    pub date: Option<i64>,
    /// RFC 2822 `Message-ID`.
    #[serde(default)]
    pub message_id: Option<String>,
    /// Total expected patches.
    #[serde(default)]
    pub total_parts: Option<u32>,
    /// Patches received.
    #[serde(default)]
    pub received_parts: Option<u32>,
    /// Subsystem tags.
    #[serde(default)]
    pub subsystems: Vec<String>,
    /// Git baseline reference.
    #[serde(default)]
    pub baseline: Option<Baseline>,
    /// Baseline application logs.
    #[serde(default)]
    pub baseline_logs: Option<String>,
    /// Individual patches in this patchset.
    #[serde(default)]
    pub patches: Vec<Patch>,
    /// AI reviews for patches in this patchset.
    #[serde(default)]
    pub reviews: Vec<Review>,
    /// Email thread messages.
    #[serde(default)]
    pub thread: Vec<ThreadMessage>,
    /// LLM model used.
    #[serde(default)]
    pub model_name: Option<String>,
    /// LLM provider.
    #[serde(default)]
    pub provider: Option<String>,
    /// Reason for failure, if applicable.
    #[serde(default)]
    pub failed_reason: Option<String>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn patchset_status_default() {
        assert_eq!(PatchsetStatus::default(), PatchsetStatus::Incomplete);
    }

    #[test]
    fn patchset_status_display() {
        assert_eq!(PatchsetStatus::InReview.to_string(), "In Review");
        assert_eq!(PatchsetStatus::FailedToApply.to_string(), "Failed to Apply");
        assert_eq!(PatchsetStatus::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn finding_counts_total() {
        let fc = FindingCounts {
            low: 2,
            medium: 0,
            high: 1,
            critical: 0,
        };
        assert_eq!(fc.total(), 3);
    }

    #[test]
    fn finding_counts_max_severity() {
        let fc = FindingCounts {
            low: 2,
            medium: 0,
            high: 1,
            critical: 0,
        };
        assert_eq!(fc.max_severity(), Some(Severity::High));

        let empty = FindingCounts::default();
        assert_eq!(empty.max_severity(), None);
    }

    #[test]
    fn finding_counts_is_empty() {
        assert!(FindingCounts::default().is_empty());
        assert!(
            !FindingCounts {
                low: 1,
                ..Default::default()
            }
            .is_empty()
        );
    }

    #[test]
    fn patchset_accessor_defaults() {
        let ps = Patchset {
            id: 1,
            subject: None,
            status: PatchsetStatus::Pending,
            thread_id: None,
            author: None,
            date: None,
            message_id: None,
            total_parts: None,
            received_parts: None,
            subsystems: vec![],
            findings: FindingCounts::default(),
            baseline_id: None,
            failed_reason: None,
            model_name: None,
            provider: None,
        };
        assert_eq!(ps.author(), "(unknown)");
        assert_eq!(ps.subject(), "(no subject)");
    }

    #[test]
    fn patchset_accessor_with_values() {
        let ps = Patchset {
            id: 1,
            subject: Some("fix null deref".to_string()),
            status: PatchsetStatus::Reviewed,
            thread_id: None,
            author: Some("dev@example.com".to_string()),
            date: None,
            message_id: None,
            total_parts: None,
            received_parts: None,
            subsystems: vec![],
            findings: FindingCounts::default(),
            baseline_id: None,
            failed_reason: None,
            model_name: None,
            provider: None,
        };
        assert_eq!(ps.author(), "dev@example.com");
        assert_eq!(ps.subject(), "fix null deref");
    }

    #[test]
    fn deserialize_patchset_from_json() {
        let json = r#"{
            "id": 19555,
            "subject": "[PATCH v2 0/4] iio: light: fix null pointer",
            "status": "Pending",
            "author": "dev@example.com",
            "date": 1778690980,
            "total_parts": 4,
            "received_parts": 4,
            "subsystems": ["LKML", "linux-iio"],
            "findings_low": 0,
            "findings_medium": 1,
            "findings_high": 0,
            "findings_critical": 0
        }"#;
        let ps: Patchset = serde_json::from_str(json).expect("deserialize patchset");
        assert_eq!(ps.status, PatchsetStatus::Pending);
        assert_eq!(ps.findings.medium, 1);
        assert_eq!(ps.findings.total(), 1);
        assert_eq!(ps.subsystems, vec!["LKML", "linux-iio"]);
    }

    #[test]
    fn deserialize_unknown_status() {
        let json = r#"{"id": 1, "status": "SomeNewStatus"}"#;
        let ps: Patchset = serde_json::from_str(json).expect("deserialize unknown status");
        assert_eq!(ps.status, PatchsetStatus::Unknown);
    }

    #[test]
    fn deserialize_null_findings() {
        let json = r#"{
            "id": 1,
            "findings_low": 2,
            "findings_medium": null,
            "findings_high": 1,
            "findings_critical": 0
        }"#;
        let ps: Patchset = serde_json::from_str(json).expect("deserialize null findings");
        assert_eq!(ps.findings.low, 2);
        assert_eq!(ps.findings.medium, 0);
        assert_eq!(ps.findings.high, 1);
        assert_eq!(ps.findings.critical, 0);
        assert_eq!(ps.findings.total(), 3);
        assert_eq!(ps.findings.max_severity(), Some(Severity::High));
    }

    #[test]
    fn deserialize_patchset_detail() {
        let json = r#"{
            "id": 123,
            "subject": "test",
            "status": "Reviewed",
            "patches": [{"id": 1, "part_index": 1}],
            "reviews": [{"id": 1, "patch_id": 1, "status": "Reviewed"}],
            "thread": [{"id": 1, "message_id": "msg@example.com"}],
            "baseline": {"branch": "main", "commit": "abc123"}
        }"#;
        let detail: PatchsetDetail =
            serde_json::from_str(json).expect("deserialize patchset detail");
        assert_eq!(detail.patches.len(), 1);
        assert_eq!(detail.reviews.len(), 1);
        assert_eq!(detail.thread.len(), 1);
        assert!(detail.baseline.is_some());
    }
}
