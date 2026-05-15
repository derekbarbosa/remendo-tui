//! Mock client for unit testing.
//!
//! Implements `SashikoApi` with canned responses that can be
//! configured per test via `set_*()` methods.

use crate::client::api::SashikoApi;
use crate::client::error::ApiError;
use crate::client::types::{ListParams, ReviewQuery};
use crate::models::{
    EmailMessage, MailingList, Paginated, PatchId, Patchset, PatchsetDetail, ServerStats,
};
use std::sync::Mutex;

type MockResult<T> = Option<Result<T, String>>;

/// A mock implementation of `SashikoApi` for testing.
///
/// Set canned responses via `set_*()` methods before calling
/// the trait methods. Unset endpoints return a descriptive error.
#[allow(clippy::struct_field_names)]
pub struct MockClient {
    patchsets_response: Mutex<MockResult<Paginated<Patchset>>>,
    stats_response: Mutex<MockResult<ServerStats>>,
    lists_response: Mutex<MockResult<Vec<MailingList>>>,
    patch_detail_response: Mutex<MockResult<PatchsetDetail>>,
    messages_response: Mutex<MockResult<Paginated<EmailMessage>>>,
    message_detail_response: Mutex<MockResult<EmailMessage>>,
}

impl MockClient {
    /// Create a new mock client with no canned responses.
    #[must_use]
    pub fn new() -> Self {
        Self {
            patchsets_response: Mutex::new(None),
            stats_response: Mutex::new(None),
            lists_response: Mutex::new(None),
            patch_detail_response: Mutex::new(None),
            messages_response: Mutex::new(None),
            message_detail_response: Mutex::new(None),
        }
    }

    /// Set the canned response for `patchsets()`.
    pub fn set_patchsets(&self, result: Result<Paginated<Patchset>, String>) {
        if let Ok(mut guard) = self.patchsets_response.lock() {
            *guard = Some(result);
        }
    }

    /// Set the canned response for `stats()`.
    pub fn set_stats(&self, result: Result<ServerStats, String>) {
        if let Ok(mut guard) = self.stats_response.lock() {
            *guard = Some(result);
        }
    }

    /// Set the canned response for `lists()`.
    pub fn set_lists(&self, result: Result<Vec<MailingList>, String>) {
        if let Ok(mut guard) = self.lists_response.lock() {
            *guard = Some(result);
        }
    }

    /// Set the canned response for `patch_detail()`.
    pub fn set_patch_detail(&self, result: Result<PatchsetDetail, String>) {
        if let Ok(mut guard) = self.patch_detail_response.lock() {
            *guard = Some(result);
        }
    }

    /// Set the canned response for `messages()`.
    pub fn set_messages(&self, result: Result<Paginated<EmailMessage>, String>) {
        if let Ok(mut guard) = self.messages_response.lock() {
            *guard = Some(result);
        }
    }

    /// Set the canned response for `message_detail()`.
    pub fn set_message_detail(&self, result: Result<EmailMessage, String>) {
        if let Ok(mut guard) = self.message_detail_response.lock() {
            *guard = Some(result);
        }
    }

    /// Extract a canned result, converting the String error to `ApiError`.
    fn take_result<T: Clone>(lock: &Mutex<MockResult<T>>, endpoint: &str) -> Result<T, ApiError> {
        let guard = lock
            .lock()
            .map_err(|e| ApiError::Configuration(format!("mock lock poisoned: {e}")))?;

        match guard.as_ref() {
            Some(Ok(val)) => Ok(val.clone()),
            Some(Err(msg)) => Err(ApiError::Network {
                source: msg.clone().into(),
                remote: "mock".to_string(),
            }),
            None => Err(ApiError::Configuration(format!(
                "mock: no canned response set for {endpoint}"
            ))),
        }
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SashikoApi for MockClient {
    async fn lists(&self) -> Result<Vec<MailingList>, ApiError> {
        Self::take_result(&self.lists_response, "lists")
    }

    async fn patchsets(&self, _params: &ListParams) -> Result<Paginated<Patchset>, ApiError> {
        Self::take_result(&self.patchsets_response, "patchsets")
    }

    async fn messages(&self, _params: &ListParams) -> Result<Paginated<EmailMessage>, ApiError> {
        Self::take_result(&self.messages_response, "messages")
    }

    async fn patch_detail(&self, _id: &PatchId) -> Result<PatchsetDetail, ApiError> {
        Self::take_result(&self.patch_detail_response, "patch_detail")
    }

    async fn patchset_summary(&self, _id: &PatchId) -> Result<PatchsetDetail, ApiError> {
        Err(ApiError::Configuration(
            "mock: patchset_summary not implemented".to_string(),
        ))
    }

    async fn message_detail(&self, _id: &PatchId) -> Result<EmailMessage, ApiError> {
        Self::take_result(&self.message_detail_response, "message_detail")
    }

    async fn review(&self, _params: &ReviewQuery) -> Result<serde_json::Value, ApiError> {
        Err(ApiError::Configuration(
            "mock: review not implemented".to_string(),
        ))
    }

    async fn review_log(&self, _params: &ReviewQuery) -> Result<serde_json::Value, ApiError> {
        Err(ApiError::Configuration(
            "mock: review_log not implemented".to_string(),
        ))
    }

    async fn stats(&self) -> Result<ServerStats, ApiError> {
        Self::take_result(&self.stats_response, "stats")
    }

    async fn stats_timeline(
        &self,
        _subsystem_id: Option<i64>,
    ) -> Result<serde_json::Value, ApiError> {
        Err(ApiError::Configuration(
            "mock: stats_timeline not implemented".to_string(),
        ))
    }

    async fn stats_reviews(&self) -> Result<serde_json::Value, ApiError> {
        Err(ApiError::Configuration(
            "mock: stats_reviews not implemented".to_string(),
        ))
    }

    async fn stats_tools(&self) -> Result<serde_json::Value, ApiError> {
        Err(ApiError::Configuration(
            "mock: stats_tools not implemented".to_string(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::models::Patchset;

    #[tokio::test]
    async fn mock_returns_canned_patchsets() {
        let mock = MockClient::new();
        let patchsets = Paginated {
            items: vec![Patchset::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        };
        mock.set_patchsets(Ok(patchsets));

        let result = mock.patchsets(&ListParams::default()).await;
        assert!(result.is_ok());
        assert_eq!(result.expect("patchsets").items.len(), 1);
    }

    #[tokio::test]
    async fn mock_returns_canned_error() {
        let mock = MockClient::new();
        mock.set_patchsets(Err("connection refused".to_string()));

        let result = mock.patchsets(&ListParams::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_returns_error_when_not_set() {
        let mock = MockClient::new();
        let result = mock.patchsets(&ListParams::default()).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ApiError::Configuration(_))));
    }

    #[tokio::test]
    async fn mock_stats() {
        let mock = MockClient::new();
        mock.set_stats(Ok(ServerStats {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            pending: 5,
            reviewing: 2,
            messages: 1000,
            patchsets: 500,
        }));

        let stats = mock.stats().await.expect("stats");
        assert_eq!(stats.status, "ok");
        assert_eq!(stats.pending, 5);
    }
}
