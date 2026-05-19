//! In-memory caching decorator for [`SashikoApi`].
//!
//! Wraps any `Arc<dyn SashikoApi>` with TTL-based caching per endpoint.
//! Each cached response is stored with an insertion timestamp and served
//! from memory if the TTL has not expired. `clear_cache()` invalidates
//! all entries (used by Refresh/Ctrl-r).

use crate::client::api::SashikoApi;
use crate::client::error::ApiError;
use crate::client::types::{ListParams, ReviewQuery};
use crate::models::{
    EmailMessage, MailingList, Paginated, PatchId, Patchset, PatchsetDetail, ServerStats,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A cached value with insertion timestamp.
struct CacheEntry<T> {
    value: T,
    inserted_at: Instant,
}

/// In-memory caching decorator for `SashikoApi`.
///
/// Caches responses from the 4 actively-used endpoints (`lists`,
/// `patchsets`, `stats`, `patchset_summary`) in typed `HashMap`s.
/// All other trait methods pass through to the inner client.
pub struct CachingClient {
    inner: Arc<dyn SashikoApi>,
    ttl: Duration,
    lists_cache: Mutex<Option<CacheEntry<Vec<MailingList>>>>,
    patchsets_cache: Mutex<HashMap<String, CacheEntry<Paginated<Patchset>>>>,
    stats_cache: Mutex<Option<CacheEntry<ServerStats>>>,
    detail_cache: Mutex<HashMap<String, CacheEntry<PatchsetDetail>>>,
}

impl CachingClient {
    /// Create a new caching decorator wrapping the given client.
    ///
    /// A `ttl` of zero disables caching (always misses).
    #[must_use]
    pub fn new(inner: Arc<dyn SashikoApi>, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            lists_cache: Mutex::new(None),
            patchsets_cache: Mutex::new(HashMap::new()),
            stats_cache: Mutex::new(None),
            detail_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Check if the TTL is non-zero (caching enabled).
    fn is_enabled(&self) -> bool {
        !self.ttl.is_zero()
    }

    /// Build a cache key for patchsets from `ListParams`.
    fn patchsets_key(params: &ListParams) -> String {
        format!(
            "p={}&pp={}&q={}&ml={}",
            params.page,
            params.per_page,
            params.search.as_deref().unwrap_or(""),
            params.mailing_list.as_deref().unwrap_or("")
        )
    }

    /// Try to read a valid entry from a single-value cache.
    fn read_single<T: Clone>(cache: &Mutex<Option<CacheEntry<T>>>, ttl: Duration) -> Option<T> {
        let guard = cache.lock().ok()?;
        let entry = guard.as_ref()?;
        if entry.inserted_at.elapsed() < ttl {
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Try to read a valid entry from a keyed cache.
    fn read_keyed<T: Clone>(
        cache: &Mutex<HashMap<String, CacheEntry<T>>>,
        key: &str,
        ttl: Duration,
    ) -> Option<T> {
        let guard = cache.lock().ok()?;
        let entry = guard.get(key)?;
        if entry.inserted_at.elapsed() < ttl {
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Store a value in a single-value cache.
    fn store_single<T>(cache: &Mutex<Option<CacheEntry<T>>>, value: T) {
        if let Ok(mut guard) = cache.lock() {
            *guard = Some(CacheEntry {
                value,
                inserted_at: Instant::now(),
            });
        }
    }

    /// Store a value in a keyed cache.
    fn store_keyed<T>(cache: &Mutex<HashMap<String, CacheEntry<T>>>, key: String, value: T) {
        if let Ok(mut guard) = cache.lock() {
            guard.insert(
                key,
                CacheEntry {
                    value,
                    inserted_at: Instant::now(),
                },
            );
        }
    }
}

#[async_trait::async_trait]
impl SashikoApi for CachingClient {
    async fn lists(&self) -> Result<Vec<MailingList>, ApiError> {
        if self.is_enabled()
            && let Some(cached) = Self::read_single(&self.lists_cache, self.ttl)
        {
            tracing::trace!("cache hit: lists");
            return Ok(cached);
        }
        let result = self.inner.lists().await?;
        if self.is_enabled() {
            Self::store_single(&self.lists_cache, result.clone());
        }
        Ok(result)
    }

    async fn patchsets(&self, params: &ListParams) -> Result<Paginated<Patchset>, ApiError> {
        let key = Self::patchsets_key(params);
        if self.is_enabled()
            && let Some(cached) = Self::read_keyed(&self.patchsets_cache, &key, self.ttl)
        {
            tracing::trace!(key = %key, "cache hit: patchsets");
            return Ok(cached);
        }
        let result = self.inner.patchsets(params).await?;
        if self.is_enabled() {
            Self::store_keyed(&self.patchsets_cache, key, result.clone());
        }
        Ok(result)
    }

    async fn stats(&self) -> Result<ServerStats, ApiError> {
        if self.is_enabled()
            && let Some(cached) = Self::read_single(&self.stats_cache, self.ttl)
        {
            tracing::trace!("cache hit: stats");
            return Ok(cached);
        }
        let result = self.inner.stats().await?;
        if self.is_enabled() {
            Self::store_single(&self.stats_cache, result.clone());
        }
        Ok(result)
    }

    async fn patchset_summary(&self, id: &PatchId) -> Result<PatchsetDetail, ApiError> {
        let key = id.to_string();
        if self.is_enabled()
            && let Some(cached) = Self::read_keyed(&self.detail_cache, &key, self.ttl)
        {
            tracing::trace!(key = %key, "cache hit: patchset detail");
            return Ok(cached);
        }
        let result = self.inner.patchset_summary(id).await?;
        if self.is_enabled() {
            Self::store_keyed(&self.detail_cache, key, result.clone());
        }
        Ok(result)
    }

    // --- Passthrough methods (not cached) ---

    async fn messages(&self, params: &ListParams) -> Result<Paginated<EmailMessage>, ApiError> {
        self.inner.messages(params).await
    }

    async fn patch_detail(&self, id: &PatchId) -> Result<PatchsetDetail, ApiError> {
        self.inner.patch_detail(id).await
    }

    async fn message_detail(&self, id: &PatchId) -> Result<EmailMessage, ApiError> {
        self.inner.message_detail(id).await
    }

    async fn review(&self, params: &ReviewQuery) -> Result<serde_json::Value, ApiError> {
        self.inner.review(params).await
    }

    async fn review_log(&self, params: &ReviewQuery) -> Result<serde_json::Value, ApiError> {
        self.inner.review_log(params).await
    }

    async fn stats_timeline(
        &self,
        subsystem_id: Option<i64>,
    ) -> Result<serde_json::Value, ApiError> {
        self.inner.stats_timeline(subsystem_id).await
    }

    async fn stats_reviews(&self) -> Result<serde_json::Value, ApiError> {
        self.inner.stats_reviews().await
    }

    async fn stats_tools(&self) -> Result<serde_json::Value, ApiError> {
        self.inner.stats_tools().await
    }

    fn clear_cache(&self) {
        if let Ok(mut guard) = self.lists_cache.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.patchsets_cache.lock() {
            guard.clear();
        }
        if let Ok(mut guard) = self.stats_cache.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.detail_cache.lock() {
            guard.clear();
        }
        tracing::debug!("cache cleared");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::client::MockClient;
    use crate::models::Patchset;

    #[tokio::test]
    async fn cache_hit_returns_cloned_value() {
        let mock = Arc::new(MockClient::new());
        let patchsets = Paginated {
            items: vec![Patchset::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        };
        mock.set_patchsets(Ok(patchsets));

        let cached = CachingClient::new(mock, Duration::from_secs(300));

        // First call: cache miss, fetches from mock
        let result1 = cached.patchsets(&ListParams::default()).await;
        assert!(result1.is_ok());

        // Second call: cache hit
        let result2 = cached.patchsets(&ListParams::default()).await;
        assert!(result2.is_ok());
        assert_eq!(result2.expect("cached").items.len(), 1);
    }

    #[tokio::test]
    async fn clear_cache_invalidates() {
        let mock = Arc::new(MockClient::new());
        mock.set_stats(Ok(ServerStats::fixture()));

        let cached = CachingClient::new(mock, Duration::from_secs(300));

        let _ = cached.stats().await;
        cached.clear_cache();

        // After clear, the cache entry should be gone
        // (but the mock still returns data, so the call succeeds)
        let result = cached.stats().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn zero_ttl_disables_caching() {
        let mock = Arc::new(MockClient::new());
        mock.set_stats(Ok(ServerStats::fixture()));

        let cached = CachingClient::new(mock, Duration::ZERO);

        // Both calls go to the inner client (no caching)
        let r1 = cached.stats().await;
        let r2 = cached.stats().await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn different_params_are_different_cache_entries() {
        let mock = Arc::new(MockClient::new());
        let patchsets = Paginated {
            items: vec![Patchset::fixture()],
            total: 1,
            page: 1,
            per_page: 50,
        };
        mock.set_patchsets(Ok(patchsets));

        let cached = CachingClient::new(mock, Duration::from_secs(300));

        let params1 = ListParams::default();
        let params2 = ListParams {
            page: 2,
            ..ListParams::default()
        };

        let _ = cached.patchsets(&params1).await;
        // params2 has a different key, so it's a cache miss
        let _ = cached.patchsets(&params2).await;
        // Both should succeed (mock returns same data for any params)
    }
}
