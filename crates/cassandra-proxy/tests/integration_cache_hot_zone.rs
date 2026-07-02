//! REALIGNMENT Phase I PR-I7 — cache hot zone non-mutation tests.
//!
//! Per `REALIGNMENT/11-phase-I-test-infra.md`, this proves that nothing
//! — live-zone compression, tool/schema sorting, cache_control
//! auto-placement — mutates the cache hot zone (`system`, `tools`,
//! frozen `messages` prefix, and content types that must round-trip
//! verbatim: thinking signatures, redacted-thinking data, and the
//! OpenAI Responses opaque types) *even while compression is actively
//! running elsewhere in the same request*.
//!
//! This is a different invariant angle than PR-I1's byte-faithful
//! round-trip tests (`integration_byte_faithful.rs`), which use
//! `CompressionMode::Off` / auth-mode-forced-off to prove *nothing*
//! mutates. Here `CompressionMode::LiveZone` is on and each test
//! includes a large compressible `tool_result` specifically so the
//! test also proves the live zone *did* shrink — a hot-zone assertion
//! that passes only because nothing ran, rather than because nothing
//! could, would be a false sense of safety.
//!
//! Four of these tests consolidate coverage that already exists in
//! `integration_responses.rs` (reasoning/compaction/v4a/local_shell —
//! OpenAI Responses opaque item types, which don't have a live-zone
//! "shrink elsewhere" angle to add since those tests already run under
//! `CompressionMode::LiveZone` against payloads with no compressible
//! content); they're re-implemented here, not just referenced, so this
//! file is a complete, standalone `make test-cache-hot-zone`-style
//! entry point per the spec.

mod common;

use common::start_proxy_with;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// A large, homogeneous JSON array — big enough to clear the
/// live-zone compression size threshold, structured enough for
/// SmartCrusher to meaningfully shrink. Mirrors the pattern already
/// proven in `integration_chat_completions.rs::compressible_tool_array_payload`.
fn compressible_tool_result_text() -> String {
    let rows: Vec<Value> = (0..1500)
        .map(|i| {
            json!({
                "id": i,
                "kind": "row",
                "value": format!("repeat-{}", i % 5),
                "status": "ok",
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap()
}

/// Build an Anthropic `/v1/messages` payload with a rich `system` +
/// `tools` (the hot zone) plus a large compressible `tool_result` in
/// the latest message (the live zone). `frozen_message` lets a test
/// insert a `cache_control`-marked early message to also exercise the
/// frozen-prefix boundary; pass `None` to omit it.
fn hot_zone_payload(frozen_message: Option<Value>) -> Value {
    let mut messages = vec![json!({
        "role": "user",
        "content": [
            {
                "type": "text",
                "text": "Find me the Q3 2024 earnings summary. 日本語 markets too. 🔥"
            }
        ]
    })];
    if let Some(m) = frozen_message {
        messages.push(m);
    }
    messages.push(json!({
        "role": "assistant",
        "content": [
            {
                "type": "tool_use",
                "id": "toolu_01XR8Q9z5w7vT3pK2nJ4hL5m",
                "name": "search_documents",
                "input": {"query": "Acme Corp Q3 2024 earnings"}
            }
        ]
    }));
    messages.push(json!({
        "role": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": "toolu_01XR8Q9z5w7vT3pK2nJ4hL5m",
                "content": compressible_tool_result_text()
            }
        ]
    }));

    json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 4096,
        "system": [
            {
                "type": "text",
                "text": "You are a careful research assistant. Use the search tool when needed.",
                "cache_control": {"type": "ephemeral"}
            },
            {
                "type": "text",
                "text": "Always cite sources. Never speculate when uncertain."
            }
        ],
        "tools": [
            {
                "name": "search_documents",
                "description": "Search the document corpus by query string. Returns up to 10 hits.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Free-text query."},
                        "limit": {"type": "integer", "minimum": 1, "maximum": 10, "default": 5}
                    },
                    "required": ["query"]
                },
                "cache_control": {"type": "ephemeral"}
            }
        ],
        "messages": messages,
    })
}

/// Send `payload` through a LiveZone-compression proxy and return
/// (inbound bytes, upstream-received bytes). Asserts 200 and that the
/// live zone actually shrank the body — the shared proof-of-liveness
/// every test in this file relies on.
async fn send_and_assert_shrunk(payload: &Value) -> (Vec<u8>, Vec<u8>) {
    let upstream = MockServer::start().await;
    let captured = mount_capture(&upstream, "/v1/messages").await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.compression = true;
        c.compression_mode = cassandra_proxy::config::CompressionMode::LiveZone;
    })
    .await;

    let body = serde_json::to_vec(payload).unwrap();
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
    assert!(
        got.len() < body.len(),
        "live zone should have shrunk the body: in={}, out={} -- if this fails, the \
         hot-zone assertions below would pass vacuously (nothing ran) rather than \
         meaningfully (hot zone survived active compression)",
        body.len(),
        got.len()
    );
    proxy.shutdown().await;
    (body, got)
}

#[tokio::test]
async fn system_byte_equal_under_compression() {
    let payload = hot_zone_payload(None);
    let (body, got) = send_and_assert_shrunk(&payload).await;

    let inbound: Value = serde_json::from_slice(&body).unwrap();
    let outbound: Value = serde_json::from_slice(&got).unwrap();
    assert_eq!(
        inbound["system"], outbound["system"],
        "system field must be byte-identical (value-equal, order-preserved) despite \
         live-zone compression elsewhere in the request"
    );
}

#[tokio::test]
async fn tools_byte_equal_under_compression() {
    let payload = hot_zone_payload(None);
    let (body, got) = send_and_assert_shrunk(&payload).await;

    let inbound: Value = serde_json::from_slice(&body).unwrap();
    let outbound: Value = serde_json::from_slice(&got).unwrap();
    assert_eq!(
        inbound["tools"], outbound["tools"],
        "tools field must be byte-identical despite live-zone compression elsewhere"
    );
}

#[tokio::test]
async fn frozen_messages_byte_equal_under_compression() {
    // cache_control on messages[0] freezes messages[0..=0] (PR-A4's
    // compute_frozen_count: highest marked index + 1). The compressible
    // tool_result lands after the boundary, in the live zone.
    let frozen_user_turn = json!({
        "role": "user",
        "content": [
            {
                "type": "text",
                "text": "This turn must never be touched once cached.",
                "cache_control": {"type": "ephemeral"}
            }
        ]
    });
    let payload = hot_zone_payload(Some(frozen_user_turn.clone()));
    let (body, got) = send_and_assert_shrunk(&payload).await;

    let inbound: Value = serde_json::from_slice(&body).unwrap();
    let outbound: Value = serde_json::from_slice(&got).unwrap();
    // messages[0] is the always-present opening user turn; messages[1]
    // is the frozen turn inserted by hot_zone_payload when Some(..).
    assert_eq!(
        inbound["messages"][0], outbound["messages"][0],
        "messages[0] (before the frozen boundary) must be byte-identical"
    );
    assert_eq!(
        inbound["messages"][1], outbound["messages"][1],
        "messages[1] (the cache_control-marked frozen turn itself) must be byte-identical"
    );
    assert_eq!(
        outbound["messages"][1], frozen_user_turn,
        "frozen turn must match exactly what was sent, not a reconstructed equivalent"
    );
}

#[tokio::test]
async fn thinking_signature_byte_equal() {
    // Thinking block + signature in an early (non-live-zone) turn;
    // compressible tool_result later. The signature is a real-shaped
    // opaque token -- must round-trip byte-for-byte, never
    // re-encoded or truncated.
    let signature = "ErcBCkgIBhABGAIiQO5fJk0wY2J3aDQ4ckZmZE5Ld2lDV3VYV1JlVlVQQUtpa3lXQVdqREZSc1Y3WkRSWjJsdndPbVlEY1ZNUUUSDDNjMjUwYWY5LWFlMmUaDDIwMjQtMTAtMjJUMjAiKjAyOjAuNjQyNDY1ODYzKtQQk19uH0K8MzUvP1ojZ2pP";
    let thinking_turn = json!({
        "role": "assistant",
        "content": [
            {
                "type": "thinking",
                "thinking": "I should call search_documents with the exact Q3 2024 query.",
                "signature": signature
            }
        ]
    });
    let payload = hot_zone_payload(Some(thinking_turn.clone()));
    let (body, got) = send_and_assert_shrunk(&payload).await;

    let inbound: Value = serde_json::from_slice(&body).unwrap();
    let outbound: Value = serde_json::from_slice(&got).unwrap();
    assert_eq!(
        inbound["messages"][1], outbound["messages"][1],
        "thinking block + signature must be byte-identical"
    );
    assert_eq!(
        outbound["messages"][1]["content"][0]["signature"],
        json!(signature)
    );
}

#[tokio::test]
async fn redacted_thinking_data_byte_equal() {
    let redacted_data = "EsADCkYIBxABGAIiQGtHMHA0QzlpbXJyV2I4QmtuS1JmTjFvUHFwS1NXa1d3Z3FVSlJSc3JKWmhLbDF3WmZmZjJyVTFqUlRYZ0FzSE0SDDk4N2MzMzgyLWFmYjAaDDIwMjQtMTAtMjJUMjAi";
    let redacted_turn = json!({
        "role": "assistant",
        "content": [
            {
                "type": "redacted_thinking",
                "data": redacted_data
            }
        ]
    });
    let payload = hot_zone_payload(Some(redacted_turn.clone()));
    let (body, got) = send_and_assert_shrunk(&payload).await;

    let inbound: Value = serde_json::from_slice(&body).unwrap();
    let outbound: Value = serde_json::from_slice(&got).unwrap();
    assert_eq!(
        inbound["messages"][1], outbound["messages"][1],
        "redacted_thinking data must be byte-identical, never partially decoded/re-encoded"
    );
    assert_eq!(
        outbound["messages"][1]["content"][0]["data"],
        json!(redacted_data)
    );
}

// ---- OpenAI Responses opaque-type hot zone (consolidated from
// integration_responses.rs; see the module doc comment for why these
// are re-implemented here rather than only referenced). ----

const V4A_DIFF: &str = "*** Begin Patch\n*** Update File: src/main.rs\n@@ -1,3 +1,4 @@\n fn main() {\n+    println!(\"hello\");\n     run();\n }\n*** End Patch\n";

async fn responses_hot_zone_item_byte_equal(item: Value) {
    let upstream = MockServer::start().await;
    let captured = mount_capture(&upstream, "/v1/responses").await;
    let proxy = start_proxy_with(&upstream.uri(), |c| {
        c.compression = true;
        c.compression_mode = cassandra_proxy::config::CompressionMode::LiveZone;
    })
    .await;

    let payload = json!({ "model": "gpt-4o", "input": [item] });
    let body = serde_json::to_vec(&payload).unwrap();
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/responses", proxy.url()))
        .header("content-type", "application/json")
        .header(
            "authorization",
            "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.signature_bytes",
        )
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let got = captured.lock().unwrap().clone().expect("upstream got body");
    let inbound: Value = serde_json::from_slice(&body).unwrap();
    let outbound: Value = serde_json::from_slice(&got).unwrap();
    assert_eq!(
        inbound["input"][0], outbound["input"][0],
        "opaque item must be byte-identical"
    );
    proxy.shutdown().await;
}

#[tokio::test]
async fn reasoning_encrypted_content_byte_equal() {
    let blob = "encrypted-reasoning-blob-".repeat(150);
    responses_hot_zone_item_byte_equal(
        json!({"type": "reasoning", "id": "r1", "encrypted_content": blob}),
    )
    .await;
}

#[tokio::test]
async fn compaction_encrypted_content_byte_equal() {
    let blob = "A".repeat(3000);
    responses_hot_zone_item_byte_equal(
        json!({"type": "compaction", "id": "k1", "encrypted_content": blob}),
    )
    .await;
}

#[tokio::test]
async fn v4a_patch_diff_byte_equal() {
    responses_hot_zone_item_byte_equal(json!({
        "type": "apply_patch_call",
        "id": "ap_1",
        "call_id": "call_1",
        "operation": {"type": "apply_patch", "diff": V4A_DIFF},
    }))
    .await;
}

#[tokio::test]
async fn local_shell_call_argv_array_preserved() {
    responses_hot_zone_item_byte_equal(json!({
        "type": "local_shell_call",
        "id": "ls_1",
        "call_id": "call_1",
        "action": {
            "type": "exec",
            "command": ["bash", "-c", "ls -la"],
            "working_directory": "/tmp",
            "timeout_ms": 60000
        }
    }))
    .await;
}
