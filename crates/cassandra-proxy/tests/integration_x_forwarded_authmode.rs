//! Phase F PR-F4 (fixes P5-53): `X-Forwarded-*`/`X-Request-Id`
//! injection must be conditional on auth mode. PAYG and OAuth keep the
//! full block; Subscription (stealth CLI/IDE traffic) skips it
//! entirely, since injecting hop-identifying headers on a request
//! meant to look like it came directly from the client is exactly the
//! fingerprint surface Subscription mode exists to avoid.
//!
//! `crates/cassandra-proxy/src/headers.rs`'s unit tests already cover
//! `build_forward_request_headers` directly; these integration tests
//! prove the same behavior end-to-end through the real proxy, with
//! auth mode driven by the same header shapes
//! `cassandra_core::auth_mode::classify` uses in production
//! (`Authorization` bearer shape / `User-Agent` prefix).

mod common;

use common::start_proxy;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn payg_adds_xfwd() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |req: &wiremock::Request| {
            assert!(
                req.headers.get("x-forwarded-for").is_some(),
                "PAYG must get x-forwarded-for"
            );
            assert!(
                req.headers.get("x-forwarded-proto").is_some(),
                "PAYG must get x-forwarded-proto"
            );
            assert!(
                req.headers.get("x-forwarded-host").is_some(),
                "PAYG must get x-forwarded-host"
            );
            assert!(
                req.headers.get("x-request-id").is_some(),
                "PAYG must get x-request-id"
            );
            ResponseTemplate::new(200).set_body_string("{}")
        })
        .mount(&upstream)
        .await;

    let proxy = start_proxy(&upstream.uri()).await;
    // x-api-key -> classify() returns AuthMode::Payg.
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/messages", proxy.url()))
        .header("x-api-key", "sk-ant-api-test")
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    proxy.shutdown().await;
}

#[tokio::test]
async fn oauth_adds_xfwd() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |req: &wiremock::Request| {
            assert!(
                req.headers.get("x-forwarded-for").is_some(),
                "OAuth must get x-forwarded-for"
            );
            assert!(
                req.headers.get("x-forwarded-proto").is_some(),
                "OAuth must get x-forwarded-proto"
            );
            assert!(
                req.headers.get("x-forwarded-host").is_some(),
                "OAuth must get x-forwarded-host"
            );
            assert!(
                req.headers.get("x-request-id").is_some(),
                "OAuth must get x-request-id"
            );
            ResponseTemplate::new(200).set_body_string("{}")
        })
        .mount(&upstream)
        .await;

    let proxy = start_proxy(&upstream.uri()).await;
    // Bearer sk-ant-oat-* -> classify() returns AuthMode::OAuth.
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/messages", proxy.url()))
        .header("authorization", "Bearer sk-ant-oat-test-token")
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    proxy.shutdown().await;
}

#[tokio::test]
async fn subscription_no_xfwd() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |req: &wiremock::Request| {
            assert!(
                req.headers.get("x-forwarded-for").is_none(),
                "Subscription must NOT get x-forwarded-for (fingerprint risk)"
            );
            assert!(
                req.headers.get("x-forwarded-proto").is_none(),
                "Subscription must NOT get x-forwarded-proto"
            );
            assert!(
                req.headers.get("x-forwarded-host").is_none(),
                "Subscription must NOT get x-forwarded-host"
            );
            assert!(
                req.headers.get("x-request-id").is_none(),
                "Subscription must NOT get x-request-id"
            );
            // Non-fingerprint headers still pass through untouched.
            assert_eq!(
                req.headers.get("authorization").unwrap(),
                "Bearer sk-ant-oat-test-token"
            );
            ResponseTemplate::new(200).set_body_string("{}")
        })
        .mount(&upstream)
        .await;

    let proxy = start_proxy(&upstream.uri()).await;
    // Subscription UA prefix wins over bearer shape even though the
    // token itself looks like an OAuth token (real-world Claude Code
    // traffic: sk-ant-oat-* carried by a claude-cli/* client).
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/messages", proxy.url()))
        .header("authorization", "Bearer sk-ant-oat-test-token")
        .header("user-agent", "claude-cli/1.2.3")
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    proxy.shutdown().await;
}
