//! `SashikoApi` trait: the async interface for Sashiko instance access.
//!
//! Uses `async_trait` for object safety — required by `RemoteManager`'s
//! `Arc<dyn SashikoApi>`.

use crate::client::error::ApiError;
use crate::client::types::{ListParams, ReviewQuery};
use crate::models::{
    EmailMessage, MailingList, Paginated, PatchId, Patchset, PatchsetDetail, ServerStats,
};

/// Read-only access to a single Sashiko instance.
///
/// All methods are async and return `Result<T, ApiError>`.
/// The trait is object-safe (via `async_trait`) to support
/// `Arc<dyn SashikoApi>` in `RemoteManager`.
#[async_trait::async_trait]
pub trait SashikoApi: Send + Sync {
    /// List tracked mailing lists.
    async fn lists(&self) -> Result<Vec<MailingList>, ApiError>;

    /// Search/list patchsets with pagination.
    async fn patchsets(&self, params: &ListParams) -> Result<Paginated<Patchset>, ApiError>;

    /// Search/list messages with pagination.
    async fn messages(&self, params: &ListParams) -> Result<Paginated<EmailMessage>, ApiError>;

    /// Full patchset detail (patches, reviews, thread, baseline).
    async fn patch_detail(&self, id: &PatchId) -> Result<PatchsetDetail, ApiError>;

    /// Patchset summary (lighter than `patch_detail`).
    async fn patchset_summary(&self, id: &PatchId) -> Result<PatchsetDetail, ApiError>;

    /// Single message detail with thread.
    async fn message_detail(&self, id: &PatchId) -> Result<EmailMessage, ApiError>;

    /// Review detail. One of `id` or `patchset_id` required.
    async fn review(&self, params: &ReviewQuery) -> Result<serde_json::Value, ApiError>;

    /// Review detail with logs.
    async fn review_log(&self, params: &ReviewQuery) -> Result<serde_json::Value, ApiError>;

    /// Server status and counts.
    async fn stats(&self) -> Result<ServerStats, ApiError>;

    /// Timeline data per day, optionally filtered by subsystem.
    async fn stats_timeline(
        &self,
        subsystem_id: Option<i64>,
    ) -> Result<serde_json::Value, ApiError>;

    /// Review statistics by provider/model/status.
    async fn stats_reviews(&self) -> Result<serde_json::Value, ApiError>;

    /// Tool usage statistics.
    async fn stats_tools(&self) -> Result<serde_json::Value, ApiError>;

    /// Clear any cached responses. Default no-op; overridden by `CachingClient`.
    fn clear_cache(&self) {}
}
