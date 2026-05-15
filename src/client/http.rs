//! Production HTTP client for Sashiko API access.

use crate::client::api::SashikoApi;
use crate::client::error::ApiError;
use crate::client::types::{ListParams, RetryConfig, ReviewQuery};
use crate::config::RemoteConfig;
use crate::models::{
    EmailMessage, MailingList, Paginated, Patchset, PatchsetDetail, PatchId, ServerStats,
};
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;

/// Production HTTP client for a single Sashiko remote.
///
/// Uses `reqwest` for async HTTP with connection pooling.
/// Each remote gets its own `HttpClient` instance.
pub struct HttpClient {
    inner: reqwest::Client,
    base_url: Url,
    remote_name: String,
    retry_config: RetryConfig,
}

impl HttpClient {
    /// Construct a new client for the given remote configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Configuration`] if the URL is invalid or
    /// the HTTP client cannot be built.
    pub fn new(config: &RemoteConfig) -> Result<Self, ApiError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ApiError::Configuration(e.to_string()))?;

        let base_url =
            Url::parse(&config.url).map_err(|e| ApiError::Configuration(e.to_string()))?;

        Ok(Self {
            inner: client,
            base_url,
            remote_name: config.name.clone(),
            retry_config: RetryConfig::from_remote(config),
        })
    }

    /// Execute a GET request with retry logic and JSON deserialization.
    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, ApiError> {
        let url = self
            .base_url
            .join(path)
            .map_err(|e| ApiError::Configuration(format!("bad path '{path}': {e}")))?;

        let mut last_error = None;
        let max_attempts = self.retry_config.max_retries + 1;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = self.compute_backoff(attempt);
                tokio::time::sleep(delay).await;
            }

            let result = self.inner.get(url.clone()).query(query).send().await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        return response
                            .json::<T>()
                            .await
                            .map_err(|e| make_deser_error(&e, path));
                    }

                    let status_code = status.as_u16();

                    // Retry on 5xx server errors
                    if is_retryable_status(status_code) {
                        let body = response.text().await.ok();
                        last_error = Some(ApiError::HttpStatus {
                            status: status_code,
                            body,
                            remote: self.remote_name.clone(),
                        });
                        continue;
                    }

                    // Don't retry client errors (4xx)
                    let body = response.text().await.ok();
                    return Err(ApiError::HttpStatus {
                        status: status_code,
                        body,
                        remote: self.remote_name.clone(),
                    });
                }
                Err(e) => {
                    if e.is_timeout() {
                        return Err(ApiError::Timeout {
                            endpoint: path.to_string(),
                            duration: Duration::from_secs(15),
                        });
                    }

                    // Network error — retry
                    last_error = Some(ApiError::Network {
                        source: Box::new(e),
                        remote: self.remote_name.clone(),
                    });
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ApiError::Network {
                source: "exhausted retries with no error captured".into(),
                remote: self.remote_name.clone(),
            }
        }))
    }

    /// Compute backoff delay with jitter for the given attempt number.
    #[allow(clippy::cast_precision_loss)]
    fn compute_backoff(&self, attempt: u32) -> Duration {
        #[allow(clippy::cast_possible_wrap)]
        let exponent = attempt.saturating_sub(1).min(30) as i32;
        let factor = self.retry_config.backoff_factor.powi(exponent);
        let base = self.retry_config.base_delay_ms as f64;
        let delay_ms = (base * factor).min(self.retry_config.max_delay_ms as f64);

        // Simple jitter: +/- 25%
        let jitter_range = delay_ms * 0.25;
        let nanos = f64::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos()),
        );
        let jitter = (nanos / f64::from(u32::MAX)) * 2.0 * jitter_range - jitter_range;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ms = (delay_ms + jitter).max(0.0) as u64;
        Duration::from_millis(ms)
    }

    /// Build query parameters for list endpoints.
    fn list_query(params: &ListParams) -> Vec<(&'static str, String)> {
        let mut query = vec![
            ("page", params.page.to_string()),
            ("per_page", params.per_page.to_string()),
        ];
        if let Some(ref q) = params.search {
            query.push(("q", q.clone()));
        }
        if let Some(ref ml) = params.mailing_list {
            query.push(("mailing_list", ml.clone()));
        }
        query
    }

    /// Build the `id` query parameter from a `PatchId`.
    fn id_query(id: &PatchId) -> Vec<(&'static str, String)> {
        vec![("id", id.to_string())]
    }

    /// Build query parameters for review endpoints.
    fn review_query(params: &ReviewQuery) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(id) = params.id {
            query.push(("id", id.to_string()));
        }
        if let Some(ps_id) = params.patchset_id {
            query.push(("patchset_id", ps_id.to_string()));
        }
        query
    }
}

/// Check if an HTTP status code should trigger a retry.
const fn is_retryable_status(status: u16) -> bool {
    matches!(status, 500 | 502 | 503 | 504)
}

/// Convert a reqwest JSON error into an [`ApiError::Deserialization`].
///
/// Since `reqwest::Error` doesn't expose the inner serde error
/// directly, we create a synthetic `serde_json::Error` by parsing
/// an intentionally invalid JSON string.
fn make_deser_error(e: &reqwest::Error, endpoint: &str) -> ApiError {
    // Intentionally parse invalid JSON to produce a serde_json::Error
    // carrying the reqwest message as context.
    match serde_json::from_str::<serde_json::Value>(
        &format!("INVALID: reqwest json error: {e}"),
    ) {
        Err(serde_err) => ApiError::Deserialization {
            source: serde_err,
            endpoint: endpoint.to_string(),
        },
        // Unreachable: the string above is never valid JSON, but
        // we handle the Ok branch to satisfy the no-unwrap rule.
        Ok(_) => ApiError::Configuration(format!("unexpected deser success for: {e}")),
    }
}

#[async_trait::async_trait]
impl SashikoApi for HttpClient {
    async fn lists(&self) -> Result<Vec<MailingList>, ApiError> {
        self.get("/api/lists", &[]).await
    }

    async fn patchsets(&self, params: &ListParams) -> Result<Paginated<Patchset>, ApiError> {
        self.get("/api/patchsets", &Self::list_query(params)).await
    }

    async fn messages(&self, params: &ListParams) -> Result<Paginated<EmailMessage>, ApiError> {
        self.get("/api/messages", &Self::list_query(params)).await
    }

    async fn patch_detail(&self, id: &PatchId) -> Result<PatchsetDetail, ApiError> {
        self.get("/api/patch", &Self::id_query(id)).await
    }

    async fn patchset_summary(&self, id: &PatchId) -> Result<PatchsetDetail, ApiError> {
        self.get("/api/patchset", &Self::id_query(id)).await
    }

    async fn message_detail(&self, id: &PatchId) -> Result<EmailMessage, ApiError> {
        self.get("/api/message", &Self::id_query(id)).await
    }

    async fn review(&self, params: &ReviewQuery) -> Result<serde_json::Value, ApiError> {
        self.get("/api/review", &Self::review_query(params)).await
    }

    async fn review_log(&self, params: &ReviewQuery) -> Result<serde_json::Value, ApiError> {
        self.get("/api/review_log", &Self::review_query(params))
            .await
    }

    async fn stats(&self) -> Result<ServerStats, ApiError> {
        self.get("/api/stats", &[]).await
    }

    async fn stats_timeline(
        &self,
        subsystem_id: Option<i64>,
    ) -> Result<serde_json::Value, ApiError> {
        let query: Vec<(&str, String)> = subsystem_id
            .map(|id| vec![("subsystem_id", id.to_string())])
            .unwrap_or_default();
        self.get("/api/stats/timeline", &query).await
    }

    async fn stats_reviews(&self) -> Result<serde_json::Value, ApiError> {
        self.get("/api/stats/reviews", &[]).await
    }

    async fn stats_tools(&self) -> Result<serde_json::Value, ApiError> {
        self.get("/api/stats/tools", &[]).await
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::config::RemoteConfig;

    fn test_remote() -> RemoteConfig {
        RemoteConfig {
            name: "test".to_string(),
            url: "https://sashiko.example.com".to_string(),
            auth_env: None,
            timeout_seconds: 15,
            max_retries: 3,
        }
    }

    #[test]
    fn new_client_with_valid_config() {
        let client = HttpClient::new(&test_remote());
        assert!(client.is_ok());
    }

    #[test]
    fn new_client_with_invalid_url() {
        let config = RemoteConfig {
            name: "bad".to_string(),
            url: "not-a-url".to_string(),
            auth_env: None,
            timeout_seconds: 15,
            max_retries: 3,
        };
        let result = HttpClient::new(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ApiError::Configuration(_))));
    }

    #[test]
    fn list_query_builds_params() {
        let params = ListParams {
            page: 2,
            per_page: 25,
            search: Some("null deref".to_string()),
            mailing_list: Some("LKML".to_string()),
        };
        let query = HttpClient::list_query(&params);
        assert_eq!(query.len(), 4);
        assert_eq!(query[0], ("page", "2".to_string()));
        assert_eq!(query[1], ("per_page", "25".to_string()));
        assert_eq!(query[2], ("q", "null deref".to_string()));
        assert_eq!(query[3], ("mailing_list", "LKML".to_string()));
    }

    #[test]
    fn id_query_numeric() {
        let query = HttpClient::id_query(&PatchId::Numeric(42));
        assert_eq!(query, vec![("id", "42".to_string())]);
    }

    #[test]
    fn id_query_message_id() {
        let query = HttpClient::id_query(&PatchId::MessageId("foo@example.com".to_string()));
        assert_eq!(query, vec![("id", "foo@example.com".to_string())]);
    }

    #[test]
    fn retryable_status_codes() {
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(502));
        assert!(is_retryable_status(503));
        assert!(is_retryable_status(504));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(404));
        assert!(!is_retryable_status(200));
    }

    #[test]
    fn backoff_increases_with_attempts() {
        let client = HttpClient::new(&test_remote()).expect("build client");
        let d1 = client.compute_backoff(1);
        let d2 = client.compute_backoff(2);
        let d3 = client.compute_backoff(3);
        // Each should be roughly 2x the previous (with jitter)
        assert!(d2 > d1 / 2, "d2 ({d2:?}) should be > d1/2 ({:?})", d1 / 2);
        assert!(d3 > d2 / 2, "d3 ({d3:?}) should be > d2/2 ({:?})", d2 / 2);
    }
}
