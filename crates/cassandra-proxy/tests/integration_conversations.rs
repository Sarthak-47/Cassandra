//! Integration tests for the Conversations API
//! (`/v1/conversations*`) — Phase C PR-C4.
//!
//! Per spec PR-C4: the Conversations endpoints are
//! passthrough-with-instrumentation. Every request must reach
//! upstream byte-equal, and every response must reach the client
//! byte-equal. Compression of stored items is C5+/B-phase territory;
//! these tests pin the byte-fidelity contract through the entire
//! conversations CRUD surface.
//!
//! The `tracing_capture` module below covers PR-C4's OTHER
//! requirement (distinct from the `/v1/conversations*` CRUD surface
//! above): when a `/v1/responses` request carries `conversation:
//! {"id": "conv_..."}`, live-zone compression must be disabled
//! entirely -- see `crate::conversations::conversation_id` and its
//! call site in `compression::live_zone_responses`.

mod common;

use common::start_proxy_with;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

#[track_caller]
fn assert_byte_equal(inbound: &[u8], received: &[u8]) {
    assert_eq!(
        inbound.len(),
        received.len(),
        "byte length mismatch: client={}, upstream={}",
        inbound.len(),
        received.len()
    );
    assert_eq!(
        sha256_hex(inbound),
        sha256_hex(received),
        "SHA-256 mismatch (client vs. upstream-received)"
    );
}

/// Mount a capture-on-path handler that records the request body.
async fn mount_capture(
    upstream: &MockServer,
    method_name: &str,
    path_str: &str,
    response_body: &'static str,
) -> Arc<Mutex<Option<Vec<u8>>>> {
    let captured: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();
    Mock::given(method(method_name))
        .and(path(path_str))
        .respond_with(move |req: &wiremock::Request| {
            *captured_clone.lock().unwrap() = Some(req.body.clone());
            ResponseTemplate::new(200).set_body_string(response_body)
        })
        .mount(upstream)
        .await;
    captured
}

#[tokio::test]
async fn create_conversation_passthrough_byte_equal() {
    let upstream = MockServer::start().await;
    let captured = mount_capture(
        &upstream,
        "POST",
        "/v1/conversations",
        r#"{"id":"conv_abc","object":"conversation"}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.enable_conversations_passthrough = true;
    })
    .await;

    let payload = json!({"metadata": {"user_id": "u1"}});
    let body = serde_json::to_vec(&payload).unwrap();
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/conversations", proxy.url()))
        .header("content-type", "application/json")
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let resp_bytes = resp.bytes().await.unwrap().to_vec();
    let resp_parsed: Value = serde_json::from_slice(&resp_bytes).unwrap();
    assert_eq!(resp_parsed["id"], json!("conv_abc"));

    let got = captured.lock().unwrap().clone().expect("body captured");
    assert_byte_equal(&body, &got);
    proxy.shutdown().await;
}

#[tokio::test]
async fn get_conversation_passthrough() {
    let upstream = MockServer::start().await;
    let _captured = mount_capture(
        &upstream,
        "GET",
        "/v1/conversations/conv_xyz",
        r#"{"id":"conv_xyz","object":"conversation","metadata":{}}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    let resp = reqwest::Client::new()
        .get(format!("{}/v1/conversations/conv_xyz", proxy.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], json!("conv_xyz"));
    proxy.shutdown().await;
}

#[tokio::test]
async fn delete_conversation_passthrough() {
    let upstream = MockServer::start().await;
    let _captured = mount_capture(
        &upstream,
        "DELETE",
        "/v1/conversations/conv_to_delete",
        r#"{"id":"conv_to_delete","deleted":true}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    let resp = reqwest::Client::new()
        .delete(format!("{}/v1/conversations/conv_to_delete", proxy.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["deleted"], json!(true));
    proxy.shutdown().await;
}

#[tokio::test]
async fn update_conversation_metadata_byte_equal() {
    let upstream = MockServer::start().await;
    let captured = mount_capture(
        &upstream,
        "POST",
        "/v1/conversations/conv_42",
        r#"{"id":"conv_42","object":"conversation"}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    let payload = json!({"metadata": {"tag": "session-2026"}});
    let body = serde_json::to_vec(&payload).unwrap();
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/conversations/conv_42", proxy.url()))
        .header("content-type", "application/json")
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("body captured");
    assert_byte_equal(&body, &got);
    proxy.shutdown().await;
}

#[tokio::test]
async fn create_items_byte_equal_through_proxy() {
    let upstream = MockServer::start().await;
    let captured = mount_capture(
        &upstream,
        "POST",
        "/v1/conversations/conv_1/items",
        r#"{"object":"list","data":[{"id":"msg_1"}]}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    // Multi-item payload — the kind of body that could grow large
    // in production. Bytes must round-trip identically.
    let payload = json!({
        "items": [
            {"type": "message", "role": "user",
             "content": [{"type": "input_text", "text": "first turn"}]},
            {"type": "message", "role": "assistant",
             "content": [{"type": "output_text", "text": "first reply"}]}
        ]
    });
    let body = serde_json::to_vec(&payload).unwrap();
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/conversations/conv_1/items", proxy.url()))
        .header("content-type", "application/json")
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("body captured");
    assert_byte_equal(&body, &got);
    proxy.shutdown().await;
}

#[tokio::test]
async fn list_items_passthrough() {
    let upstream = MockServer::start().await;
    let _captured = mount_capture(
        &upstream,
        "GET",
        "/v1/conversations/conv_1/items",
        r#"{"object":"list","data":[]}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    let resp = reqwest::Client::new()
        .get(format!("{}/v1/conversations/conv_1/items", proxy.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["object"], json!("list"));
    proxy.shutdown().await;
}

#[tokio::test]
async fn get_item_passthrough() {
    let upstream = MockServer::start().await;
    let _captured = mount_capture(
        &upstream,
        "GET",
        "/v1/conversations/conv_1/items/item_42",
        r#"{"id":"item_42","type":"message"}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    let resp = reqwest::Client::new()
        .get(format!(
            "{}/v1/conversations/conv_1/items/item_42",
            proxy.url()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], json!("item_42"));
    proxy.shutdown().await;
}

#[tokio::test]
async fn delete_item_passthrough() {
    let upstream = MockServer::start().await;
    let _captured = mount_capture(
        &upstream,
        "DELETE",
        "/v1/conversations/conv_1/items/item_42",
        r#"{"id":"item_42","deleted":true}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    let resp = reqwest::Client::new()
        .delete(format!(
            "{}/v1/conversations/conv_1/items/item_42",
            proxy.url()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["deleted"], json!(true));
    proxy.shutdown().await;
}

#[tokio::test]
async fn upstream_error_surfaces_verbatim() {
    // No-silent-fallbacks: if upstream returns 4xx/5xx, we forward
    // it verbatim — never swallow + return 500.
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/conversations/missing"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_string(r#"{"error":{"message":"conversation not found"}}"#)
                .insert_header("content-type", "application/json"),
        )
        .mount(&upstream)
        .await;
    let proxy = start_proxy_with(&upstream.uri(), |_| {}).await;

    let resp = reqwest::Client::new()
        .get(format!("{}/v1/conversations/missing", proxy.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["message"], json!("conversation not found"));
    proxy.shutdown().await;
}

#[tokio::test]
async fn passthrough_disabled_falls_through_to_catch_all() {
    // When `enable_conversations_passthrough = false`, the per-route
    // axum handlers are NOT mounted, but the request still reaches
    // upstream via the catch-all. Bytes still round-trip equal.
    let upstream = MockServer::start().await;
    let captured = mount_capture(
        &upstream,
        "POST",
        "/v1/conversations",
        r#"{"id":"conv_fallthrough","object":"conversation"}"#,
    )
    .await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.enable_conversations_passthrough = false;
    })
    .await;

    let payload = json!({"metadata": {"x": 1}});
    let body = serde_json::to_vec(&payload).unwrap();
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/conversations", proxy.url()))
        .header("content-type", "application/json")
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("body captured");
    assert_byte_equal(&body, &got);
    proxy.shutdown().await;
}

// ─── PR-C4: conversation.id in a /v1/responses body disables ───────
// ─── live-zone compression entirely ─────────────────────────────────

mod conversations_api_compression_skip {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use std::sync::OnceLock;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone)]
    struct CaptureWriter {
        inner: Arc<StdMutex<Vec<u8>>>,
    }

    impl std::io::Write for CaptureWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for CaptureWriter {
        type Writer = Self;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn buffer() -> &'static Arc<StdMutex<Vec<u8>>> {
        static BUFFER: OnceLock<Arc<StdMutex<Vec<u8>>>> = OnceLock::new();
        BUFFER.get_or_init(|| {
            let buf = Arc::new(StdMutex::new(Vec::new()));
            let writer = CaptureWriter { inner: buf.clone() };
            let subscriber = tracing_subscriber::fmt()
                .json()
                .with_writer(writer)
                // INFO per the PR-C4 acceptance criterion: "The
                // Conversations API warning appears in logs at INFO
                // level with a conversation_id field."
                .with_max_level(tracing::Level::INFO)
                .finish();
            // Best-effort install: other integration test binaries in
            // this crate (e.g. integration_volatile_detector.rs) may
            // already have set a default subscriber in a separate
            // process; within this binary we only need one.
            let _ = tracing::subscriber::set_global_default(subscriber);
            buf
        })
    }

    #[tokio::test]
    async fn conversation_id_present_skips_compression_warns() {
        let buf = buffer();
        buf.lock().unwrap().clear();

        let upstream = MockServer::start().await;
        let captured = mount_capture(&upstream, "POST", "/v1/responses", r#"{"ok":true}"#).await;
        let proxy = start_proxy_with(&upstream.uri(), |c| {
            c.compression = true;
            c.compression_mode = cassandra_proxy::config::CompressionMode::LiveZone;
        })
        .await;

        // function_call_output well over the 2 KiB output-item floor
        // -- large enough that it WOULD be compressed if this weren't
        // a Conversations API request.
        let large_output = "x".repeat(4096);
        let payload = json!({
            "model": "gpt-5.1",
            "conversation": {"id": "conv_test_abc123"},
            "input": [
                {
                    "type": "function_call_output",
                    "call_id": "call_1",
                    "output": large_output,
                }
            ],
        });
        let body = serde_json::to_vec(&payload).unwrap();
        let resp = reqwest::Client::new()
            .post(format!("{}/v1/responses", proxy.url()))
            .header("content-type", "application/json")
            .body(body.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        // Give the async tracing emitter a beat to flush.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let logs = String::from_utf8(buf.lock().unwrap().clone()).expect("logs are utf-8");
        assert!(
            logs.contains("conversations_api"),
            "expected reason=conversations_api in logs; got: {logs}"
        );
        assert!(
            logs.contains("conv_test_abc123"),
            "expected the real conversation_id in logs; got: {logs}"
        );
        assert!(
            logs.contains(r#""decision":"passthrough""#),
            "expected decision=passthrough in logs; got: {logs}"
        );

        // Live-zone compression specifically did not run: the
        // function_call_output's 4096-byte payload survives intact in
        // the upstream-received bytes, even though it's well over
        // every content-type's compression threshold. (Not asserting
        // full request byte-equality here -- PAYG's independent PR-E4
        // prompt_cache_key auto-injection is expected to still apply;
        // that's unrelated to live-zone compression and not something
        // PR-C4 exempts Conversations API requests from.)
        let upstream_received = captured
            .lock()
            .unwrap()
            .clone()
            .expect("upstream should have captured a body");
        let received_str = String::from_utf8(upstream_received).expect("utf8");
        assert!(
            received_str.contains(&large_output),
            "the 4096-byte function_call_output must survive uncompressed \
             when conversation.id is present"
        );

        proxy.shutdown().await;
    }
}
