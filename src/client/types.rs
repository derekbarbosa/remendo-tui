//! Query parameter types and retry configuration for API requests.

/// Parameters for list endpoints (`/api/patchsets`, `/api/messages`).
#[derive(Debug, Clone)]
pub struct ListParams {
    /// Page number (1-indexed).
    pub page: u32,
    /// Items per page (max 100).
    pub per_page: u32,
    /// Optional search query string.
    pub search: Option<String>,
    /// Optional mailing list filter.
    pub mailing_list: Option<String>,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 50,
            search: None,
            mailing_list: None,
        }
    }
}

/// Parameters for review endpoints (`/api/review`, `/api/review_log`).
#[derive(Debug, Clone)]
pub struct ReviewQuery {
    /// Review ID (one of `id` or `patchset_id` required).
    pub id: Option<i64>,
    /// Patchset ID to get the latest review for.
    pub patchset_id: Option<i64>,
}

/// Retry configuration for transient failures.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Base delay between retries in milliseconds.
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds.
    pub max_delay_ms: u64,
    /// Backoff multiplier applied after each retry.
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 500,
            max_delay_ms: 10_000,
            backoff_factor: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a retry config from a `RemoteConfig`.
    #[must_use]
    pub fn from_remote(config: &crate::config::RemoteConfig) -> Self {
        Self {
            max_retries: config.max_retries,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_params_defaults() {
        let params = ListParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 50);
        assert!(params.search.is_none());
        assert!(params.mailing_list.is_none());
    }

    #[test]
    fn retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 10_000);
    }
}
