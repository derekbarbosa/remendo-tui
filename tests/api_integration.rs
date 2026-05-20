//! Integration tests for the HTTP API client using wiremock.

#![allow(clippy::expect_used)]

use remendo::client::{ApiError, HttpClient, ListParams, SashikoApi};
use remendo::config::RemoteConfig;
use remendo::models::{Paginated, PatchId, Patchset, PatchsetDetail, ServerStats};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

/// Create a `RemoteConfig` pointing at the given mock server URI.
fn test_remote(uri: &str) -> RemoteConfig {
    RemoteConfig {
        name: "test".to_string(),
        url: uri.to_string(),
        auth_env: None,
        timeout_seconds: 5,
        max_retries: 1,
    }
}

/// Fixture JSON for a patchsets response.
fn patchsets_json() -> serde_json::Value {
    json!({
        "items": [{
            "id": 1,
            "subject": "[PATCH] Fix null deref",
            "status": "Reviewed",
            "author": "dev@example.com",
            "date": 1_778_690_980_i64,
            "total_parts": 1,
            "received_parts": 1,
            "subsystems": ["LKML"],
            "findings_low": 0,
            "findings_medium": 1,
            "findings_high": 0,
            "findings_critical": 0
        }],
        "total": 1,
        "page": 1,
        "per_page": 50
    })
}

/// Fixture JSON for a stats response.
fn stats_json() -> serde_json::Value {
    json!({
        "status": "ok",
        "version": "0.1.6",
        "pending": 42,
        "reviewing": 3,
        "messages": 136_621,
        "patchsets": 19_558
    })
}

#[tokio::test]
async fn fetches_patchsets_successfully() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/patchsets"))
        .respond_with(ResponseTemplate::new(200).set_body_json(patchsets_json()))
        .mount(&server)
        .await;

    let client = HttpClient::new(&test_remote(&server.uri())).expect("build client");
    let result: Result<Paginated<Patchset>, ApiError> =
        client.patchsets(&ListParams::default()).await;
    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let paginated = result.expect("patchsets");
    assert_eq!(paginated.total, 1);
    assert_eq!(paginated.items.len(), 1);
    assert_eq!(paginated.items[0].author(), "dev@example.com");
}

#[tokio::test]
async fn fetches_stats_successfully() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/stats"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stats_json()))
        .mount(&server)
        .await;

    let client = HttpClient::new(&test_remote(&server.uri())).expect("build client");
    let stats: ServerStats = client.stats().await.expect("stats");
    assert_eq!(stats.status, "ok");
    assert_eq!(stats.pending, 42);
    assert_eq!(stats.patchsets, 19_558);
}

#[tokio::test]
async fn handles_404_without_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/patchsets"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let client = HttpClient::new(&test_remote(&server.uri())).expect("build client");
    let result: Result<Paginated<Patchset>, ApiError> =
        client.patchsets(&ListParams::default()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handles_500_with_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/stats"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .expect(2) // Initial + 1 retry
        .mount(&server)
        .await;

    let client = HttpClient::new(&test_remote(&server.uri())).expect("build client");
    let result: Result<ServerStats, ApiError> = client.stats().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handles_malformed_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/stats"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let client = HttpClient::new(&test_remote(&server.uri())).expect("build client");
    let result: Result<ServerStats, ApiError> = client.stats().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn fetches_patch_detail_successfully() {
    let fixture =
        std::fs::read_to_string("tests/fixtures/patch_detail.json").expect("read fixture");
    let fixture_json: serde_json::Value =
        serde_json::from_str(&fixture).expect("parse fixture JSON");

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/patch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture_json))
        .mount(&server)
        .await;

    let client = HttpClient::new(&test_remote(&server.uri())).expect("build client");
    let result: Result<PatchsetDetail, ApiError> =
        client.patch_detail(&PatchId::Numeric(19555)).await;
    assert!(result.is_ok(), "expected Ok, got: {result:?}");

    let detail = result.expect("patch detail");
    assert_eq!(detail.id, 19555);
    assert_eq!(detail.patches.len(), 2);
    assert_eq!(detail.reviews.len(), 1);
    assert_eq!(detail.thread.len(), 2);
    assert!(detail.baseline.is_some());
    assert_eq!(
        detail.subject.as_deref(),
        Some("[PATCH v2 0/4] iio: light: fix null pointer dereference")
    );
}

/// A responder that returns 500 on the first request, then 200 with the given body.
struct FailThenSucceed {
    body: serde_json::Value,
    call_count: std::sync::atomic::AtomicU32,
}

impl Respond for FailThenSucceed {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        let count = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count == 0 {
            ResponseTemplate::new(500).set_body_string("internal error")
        } else {
            ResponseTemplate::new(200).set_body_json(&self.body)
        }
    }
}

#[tokio::test]
async fn retries_on_500_then_succeeds() {
    let server = MockServer::start().await;
    let responder = FailThenSucceed {
        body: stats_json(),
        call_count: std::sync::atomic::AtomicU32::new(0),
    };

    Mock::given(method("GET"))
        .and(path("/api/stats"))
        .respond_with(responder)
        .expect(2) // initial 500 + retry 200
        .mount(&server)
        .await;

    let client = HttpClient::new(&test_remote(&server.uri())).expect("build client");
    let result: Result<ServerStats, ApiError> = client.stats().await;
    assert!(result.is_ok(), "expected Ok after retry, got: {result:?}");
    assert_eq!(result.expect("stats").status, "ok");
}
