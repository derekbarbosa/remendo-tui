//! Domain data models for Sashiko API entities.
//!
//! This module defines the shared type vocabulary consumed by the API client
//! (deserialization), the UI layer (rendering), and the cache layer (storage).
//! Types are unified — they deserialize directly from Sashiko's JSON API
//! with no separate wire/domain layer.

pub mod common;
pub mod message;
pub mod pagination;
pub mod patch;
pub mod patchset;
pub mod review;

#[cfg(test)]
pub mod fixtures;

pub use common::{Baseline, MailingList, PatchId, Severity, ServerStats};
pub use message::{EmailMessage, ThreadMessage};
pub use pagination::Paginated;
pub use patch::{Patch, PatchStatus};
pub use patchset::{FindingCounts, Patchset, PatchsetDetail, PatchsetStatus};
pub use review::{Review, ReviewStatus};
