//! Integration tests for the HTTP API client using wiremock.

#![allow(clippy::expect_used)]

use remendo_tui::client::{ApiError, HttpClient, ListParams, SashikoApi};
use remendo_tui::config::RemoteConfig;
use remendo_tui::models::{Paginated, Patchset, ServerStats};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
