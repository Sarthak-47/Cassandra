//! REALIGNMENT Phase I PR-I3 — property tests for compression invariants.
//!
//! Per `REALIGNMENT/11-phase-I-test-infra.md`, exercises the realigned
//! compressor against `cases = 1000` randomly generated Anthropic
//! `/v1/messages` request bodies (via `compress_anthropic_live_zone`,
//! the same dispatcher `crates/cassandra-proxy` calls), checking five
//! invariants:
//!
//! 1. **Determinism** — `compress(input) == compress(input)`.
//! 2. **Idempotence** — compressing already-compressed content is a
//!    no-op: `compress(compress(input).output) == compress(input).output`.
//! 3. **Token-non-increasing** — `tokens(output) <= tokens(input)`.
//! 4. **Position preservation** — message count and per-message block
//!    count/type are unchanged; `tool_use_id` values are preserved.
//! 5. **Frozen-prefix integrity** — for any `frozen_count`, messages
//!    `0..frozen_count` are byte-equal in input and output.
//!
//! The generator produces a mix of small, incompressible tool_result
//! blocks and large, structured ones that clear the live-zone size
//! threshold — proptest's shrinker will home in on the smallest
//! failing case if any invariant breaks, same as it would for a
//! hand-picked regression fixture.

use cassandra_core::tokenizer::{EstimatingCounter, Tokenizer};
use cassandra_core::transforms::live_zone::{compress_anthropic_live_zone, LiveZoneOutcome};
use cassandra_core::transforms::AuthMode;
use proptest::prelude::*;
use serde_json::{json, Value};

/// A short, human-readable text block. Never large enough to compress
/// on its own -- exists so not every block in a generated request is a
/// tool_result.
fn arb_text_block() -> impl Strategy<Value = Value> {
    "[a-zA-Z0-9 .,!?]{0,120}".prop_map(|s| json!({"type": "text", "text": s}))
}

/// A `tool_use` block with a randomized id/name -- the id must survive
/// compression unchanged (invariant 4).
fn arb_tool_use_block() -> impl Strategy<Value = Value> {
    ("[a-z_]{3,12}", "[a-zA-Z0-9]{8,20}")
        .prop_map(|(name, id)| json!({"type": "tool_use", "id": format!("toolu_{id}"), "name": name, "input": {}}))
}

/// A `tool_result` block. Two shapes, weighted so both the
/// "nothing to compress" and "genuinely shrinks" paths get exercised:
/// - small (rarely clears the size threshold)
/// - large, homogeneous JSON array (reliably clears it, matching the
///   pattern already proven in `integration_chat_completions.rs` and
///   `integration_cache_hot_zone.rs`)
fn arb_tool_result_block() -> impl Strategy<Value = Value> {
    prop_oneof![
        3 => "[a-zA-Z0-9 .,!?]{0,80}".prop_map(|s| {
            json!({"type": "tool_result", "tool_use_id": "toolu_fixed", "content": s})
        }),
        1 => (50usize..250).prop_map(|n| {
            let rows: Vec<Value> = (0..n)
                .map(|i| json!({"id": i, "kind": "row", "value": format!("repeat-{}", i % 5)}))
                .collect();
            json!({
                "type": "tool_result",
                "tool_use_id": "toolu_fixed",
                "content": serde_json::to_string(&rows).unwrap()
            })
        }),
    ]
}

fn arb_message() -> impl Strategy<Value = Value> {
    prop_oneof![
        arb_text_block().prop_map(|b| json!({"role": "user", "content": [b]})),
        arb_tool_use_block().prop_map(|b| json!({"role": "assistant", "content": [b]})),
        arb_tool_result_block().prop_map(|b| json!({"role": "user", "content": [b]})),
    ]
}

/// (body_bytes, frozen_message_count) -- the count is always a valid
/// index in `0..=messages.len()`, matching what
/// `cache_control::compute_frozen_count` would ever hand the
/// dispatcher in production.
fn arb_request() -> impl Strategy<Value = (Vec<u8>, usize)> {
    prop::collection::vec(arb_message(), 1..8).prop_flat_map(|messages| {
        let len = messages.len();
        (Just(messages), 0..=len).prop_map(|(messages, frozen)| {
            let body = json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 4096,
                "messages": messages,
            });
            (serde_json::to_vec(&body).unwrap(), frozen)
        })
    })
}

/// Resolve a `LiveZoneOutcome` to the bytes that would actually be
/// forwarded upstream: `new_body` when modified, the original input
/// otherwise (matches the proxy's own `NoChange` contract).
fn outcome_bytes(input: &[u8], outcome: &LiveZoneOutcome) -> Vec<u8> {
    match outcome {
        LiveZoneOutcome::NoChange { .. } => input.to_vec(),
        LiveZoneOutcome::Modified { new_body, .. } => new_body.get().as_bytes().to_vec(),
    }
}

fn compress(input: &[u8], frozen: usize) -> Vec<u8> {
    let outcome =
        compress_anthropic_live_zone(input, frozen, AuthMode::Payg, "claude-3-5-sonnet-20241022")
            .expect("generator only produces valid Anthropic request bodies");
    outcome_bytes(input, &outcome)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

    #[test]
    fn determinism((body, frozen) in arb_request()) {
        let a = compress(&body, frozen);
        let b = compress(&body, frozen);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn idempotence((body, frozen) in arb_request()) {
        let once = compress(&body, frozen);
        let twice = compress(&once, frozen);
        prop_assert_eq!(once, twice, "compressing already-compressed output must be a no-op");
    }

    #[test]
    fn token_non_increasing((body, frozen) in arb_request()) {
        let out = compress(&body, frozen);
        let counter = EstimatingCounter::default();
        let in_str = std::str::from_utf8(&body).unwrap();
        let out_str = std::str::from_utf8(&out).unwrap();
        prop_assert!(
            counter.count_text(out_str) <= counter.count_text(in_str),
            "output must never use more estimated tokens than input: in={} out={}",
            counter.count_text(in_str),
            counter.count_text(out_str)
        );
    }

    #[test]
    fn position_preservation((body, frozen) in arb_request()) {
        let out = compress(&body, frozen);
        let in_val: Value = serde_json::from_slice(&body).unwrap();
        let out_val: Value = serde_json::from_slice(&out).unwrap();
        let in_messages = in_val["messages"].as_array().unwrap();
        let out_messages = out_val["messages"].as_array().unwrap();
        prop_assert_eq!(in_messages.len(), out_messages.len(), "message count must be preserved");
        for (i, (im, om)) in in_messages.iter().zip(out_messages.iter()).enumerate() {
            let ic = im["content"].as_array().unwrap();
            let oc = om["content"].as_array().unwrap();
            prop_assert_eq!(ic.len(), oc.len(), "messages[{}] block count must be preserved", i);
            for (j, (ib, ob)) in ic.iter().zip(oc.iter()).enumerate() {
                prop_assert_eq!(
                    ib["type"].as_str(), ob["type"].as_str(),
                    "messages[{}].content[{}] type must be preserved", i, j
                );
                if let Some(id) = ib.get("id") {
                    prop_assert_eq!(Some(id), ob.get("id"), "tool_use id must be preserved");
                }
                if let Some(tool_use_id) = ib.get("tool_use_id") {
                    prop_assert_eq!(
                        Some(tool_use_id), ob.get("tool_use_id"),
                        "tool_result tool_use_id must be preserved"
                    );
                }
            }
        }
    }

    #[test]
    fn frozen_prefix_integrity((body, frozen) in arb_request()) {
        let out = compress(&body, frozen);
        let in_val: Value = serde_json::from_slice(&body).unwrap();
        let out_val: Value = serde_json::from_slice(&out).unwrap();
        let in_messages = in_val["messages"].as_array().unwrap();
        let out_messages = out_val["messages"].as_array().unwrap();
        for i in 0..frozen.min(in_messages.len()) {
            prop_assert_eq!(
                &in_messages[i], &out_messages[i],
                "messages[{}] is within the frozen prefix (frozen_count={}) and must be byte-identical",
                i, frozen
            );
        }
    }
}
