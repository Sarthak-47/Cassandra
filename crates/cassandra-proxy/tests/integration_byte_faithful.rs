//! REALIGNMENT Phase I PR-I1 — SHA-256 byte-faithful round-trip tests.
//!
//! Per `REALIGNMENT/11-phase-I-test-infra.md`, this is "the single most
//! important regression test for cache safety": send a real,
//! production-shaped payload through the proxy with compression off (or
//! auth-mode-forced-off), and assert the bytes that reach the upstream
//! are byte-identical (SHA-256) to what the client sent. JSON
//! value-equality is not a sound substitute — it misses whitespace, key
//! order, and Unicode-escape differences that all bust prompt cache hit
//! rate.
//!
//! This file is the canonical, single-purpose entry point for that
//! invariant (`make test-byte-faithful`) — a reader chasing a cache-hit-
//! rate regression should be able to find it here without hunting
//! through the broader `integration_compression.rs` /
//! `integration_chat_completions.rs` / `integration_responses.rs` files,
//! which cover many additional dispatcher-specific cases beyond this
//! one invariant.
//!
//! Coverage:
//!
//! - `sha256_round_trip_anthropic_messages_passthrough` — the recorded
//!   `anthropic_messages_request_real.json` fixture through `/v1/messages`
//!   with compression on (Phase A passthrough).
//! - `sha256_round_trip_anthropic_messages_compression_off_via_auth_mode`
//!   — the same fixture, but with OAuth-classifying headers, proving the
//!   PAYG/OAuth policy gate (PR-F2) doesn't perturb the body either.
//! - `sha256_round_trip_openai_chat` — the recorded
//!   `openai_chat_completions_real.json` fixture through
//!   `/v1/chat/completions`.
//! - `sha256_round_trip_openai_responses` — the recorded
//!   `openai_responses_real.json` fixture (V4A patch, local_shell_call,
//!   reasoning, compaction items) through `/v1/responses`.

mod common;

use common::start_proxy_with;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// An OAuth-shaped bearer (3 dot-separated segments) so
/// `classify_auth_mode` resolves `AuthMode::OAuth` — used to prove the
/// PAYG/OAuth policy gate doesn't touch the body on this path either.
const OAUTH_BEARER: &str = "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.signature_bytes";

/// Mount a capture handler on `endpoint_path` that records the raw
/// upstream-bound request body and returns 200 OK.
async fn mount_capture(upstream: &MockServer, endpoint_path: &str) -> Arc<Mutex<Option<Vec<u8>>>> {
    let captured: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    Mock::given(method("POST"))
        .and(path(endpoint_path))
        .respond_with(move |req: &wiremock::Request| {
            *captured_clone.lock().unwrap() = Some(req.body.clone());
            ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#)
        })
        .mount(upstream)
        .await;
    captured
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().fold(String::with_capacity(64), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

#[track_caller]
fn assert_byte_equal_sha256(inbound: &[u8], received: &[u8]) {
    let inbound_hash = sha256_hex(inbound);
    let received_hash = sha256_hex(received);
    assert_eq!(
        inbound.len(),
        received.len(),
        "byte length mismatch: inbound={}, upstream-received={}",
        inbound.len(),
        received.len(),
    );
    assert_eq!(
        inbound_hash, received_hash,
        "SHA-256 mismatch: inbound={inbound_hash}, upstream-received={received_hash}",
    );
}

fn read_fixture(name: &str) -> Vec<u8> {
    let path = format!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/{}"),
        name
    );
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} present in repo: {e}"));
    // Sanity: the fixture should parse as JSON. We never parse it
    // through the proxy — passthrough is byte-faithful — but we want
    // a clear failure if someone corrupts the file rather than a
    // confusing downstream SHA mismatch.
    let _: Value = serde_json::from_slice(&bytes).expect("fixture parses as json");
    bytes
}

#[tokio::test]
async fn sha256_round_trip_anthropic_messages_passthrough() {
    let upstream = MockServer::start().await;
    let captured = mount_capture(&upstream, "/v1/messages").await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.compression = true;
    })
    .await;

    let body = read_fixture("anthropic_messages_request_real.json");
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/messages", proxy.url()))
        .header("content-type", "application/json")
        .header("x-api-key", "sk-ant-api-test-key")
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("upstream got body");
    assert_byte_equal_sha256(&body, &got);
    proxy.shutdown().await;
}

#[tokio::test]
async fn sha256_round_trip_anthropic_messages_compression_off_via_auth_mode() {
    let upstream = MockServer::start().await;
    let captured = mount_capture(&upstream, "/v1/messages").await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.compression = true;
    })
    .await;

    let body = read_fixture("anthropic_messages_request_real.json");
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/messages", proxy.url()))
        .header("content-type", "application/json")
        .header("authorization", OAUTH_BEARER)
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("upstream got body");
    assert_byte_equal_sha256(&body, &got);
    proxy.shutdown().await;
}

#[tokio::test]
async fn sha256_round_trip_openai_chat() {
    let upstream = MockServer::start().await;
    let captured = mount_capture(&upstream, "/v1/chat/completions").await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.compression = true;
    })
    .await;

    let body = read_fixture("openai_chat_completions_real.json");
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/chat/completions", proxy.url()))
        .header("content-type", "application/json")
        .header("authorization", OAUTH_BEARER)
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("upstream got body");
    assert_byte_equal_sha256(&body, &got);
    proxy.shutdown().await;
}

#[tokio::test]
async fn sha256_round_trip_openai_responses() {
    let upstream = MockServer::start().await;
    let captured = mount_capture(&upstream, "/v1/responses").await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.compression = true;
    })
    .await;

    let body = read_fixture("openai_responses_real.json");
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/responses", proxy.url()))
        .header("content-type", "application/json")
        .header("authorization", OAUTH_BEARER)
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("upstream got body");
    assert_byte_equal_sha256(&body, &got);
    proxy.shutdown().await;
}
