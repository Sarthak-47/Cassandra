//! REALIGNMENT Phase I PR-I2 — SSE parser corner-case fixtures.
//!
//! Each fixture under `tests/fixtures/sse/` captures one wire-format
//! corner case the framer/state-machines must survive: out-of-order
//! block completion, keepalive pings, mid-stream errors, split UTF-8
//! codepoints across TCP chunk boundaries, OpenAI's positionless
//! `tool_calls` accumulation, and a stream that TCP-drops before its
//! terminal event. These are `assert`-driven (not just "doesn't
//! panic" -- that's `proptest_sse.rs`'s job) because each corner case
//! has a specific, previously-buggy expected outcome documented in
//! the module docs of `src/sse/{anthropic,openai_chat,openai_responses}.rs`.

use bytes::Bytes;
use cassandra_proxy::sse::anthropic::{AnthropicStreamState, StreamStatus as AnthropicStatus};
use cassandra_proxy::sse::openai_chat::{ChunkState, StreamStatus as OpenAiChatStatus};
use cassandra_proxy::sse::openai_responses::{ResponseState, StreamStatus as ResponsesStatus};
use cassandra_proxy::sse::SseFramer;

fn read_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sse")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {path:?}: {e}"))
}

/// Push fixture bytes into a framer in one shot and drain every event,
/// panicking loudly on a framing error (fixtures are hand-written valid
/// SSE; a `FramingError` here means the fixture itself is malformed).
fn drain_events(bytes: &[u8]) -> Vec<cassandra_proxy::sse::SseEvent> {
    let mut framer = SseFramer::new();
    framer.push(bytes);
    let mut events = Vec::new();
    while let Some(result) = framer.next_event() {
        events.push(result.expect("fixture must be well-formed SSE"));
    }
    events
}

#[test]
fn anthropic_thinking_with_signature_preserved_byte_equal() {
    let bytes = read_fixture("anthropic_thinking_with_signature.sse");
    let mut state = AnthropicStreamState::new();
    for event in drain_events(&bytes) {
        state.apply(event).expect("apply must not error");
    }

    let thinking_block = state.blocks.get(&0).expect("block 0 present");
    assert_eq!(thinking_block.block_type, "thinking");
    assert_eq!(
        thinking_block.text_buffer,
        "Let me consider the question carefully."
    );
    assert_eq!(
        thinking_block.signature.as_deref(),
        Some("EqQBCkYIARgCIkAy8f3IjSTest==")
    );
    assert!(thinking_block.complete);

    let text_block = state.blocks.get(&1).expect("block 1 present");
    assert_eq!(text_block.block_type, "text");
    assert_eq!(text_block.text_buffer, "The answer is 42.");

    assert_eq!(state.stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(state.status, AnthropicStatus::MessageStop);
}

#[test]
fn anthropic_interleaved_blocks_keyed_by_index_not_arrival_order() {
    let bytes = read_fixture("anthropic_interleaved_blocks.sse");
    let mut state = AnthropicStreamState::new();
    for event in drain_events(&bytes) {
        state.apply(event).expect("apply must not error");
    }

    // index 1 was opened first on the wire, but must land in the
    // index-1 slot, not the position-0 slot -- P1-17-class bug.
    let block0 = state.blocks.get(&0).expect("block 0 present");
    assert_eq!(block0.text_buffer, "first block text");
    assert!(block0.complete);

    let block1 = state.blocks.get(&1).expect("block 1 present");
    assert_eq!(block1.text_buffer, "second block text");
    assert!(block1.complete);

    assert_eq!(state.status, AnthropicStatus::MessageStop);
}

#[test]
fn anthropic_input_json_delta_survives_utf8_split_at_arbitrary_chunk_boundaries() {
    let bytes = read_fixture("anthropic_input_json_delta_split_utf8.sse");

    // Feed the framer in tiny, odd-sized chunks so at least some
    // splits land mid-codepoint (the emoji characters are 4-byte
    // UTF-8 sequences). This is the direct regression test for P1-15:
    // the old Python proxy's errors="ignore" chunk-wise decode
    // silently corrupted exactly this class of input.
    let mut framer = SseFramer::new();
    let mut state = AnthropicStreamState::new();
    for chunk in bytes.chunks(3) {
        framer.push(chunk);
        while let Some(result) = framer.next_event() {
            let event = result.expect("fixture must be well-formed SSE");
            state.apply(event).expect("apply must not error");
        }
    }

    let block = state.blocks.get(&0).expect("block 0 present");
    assert_eq!(
        block.partial_json,
        "{\"emoji\":\"🎉\",\"label\":\"celebrate 🎊 now\"}"
    );
    // Confirm it's valid, correctly-reassembled JSON, not just a
    // byte-equal string coincidence.
    let parsed: serde_json::Value = serde_json::from_str(&block.partial_json)
        .expect("reassembled partial_json must parse as JSON");
    assert_eq!(parsed["emoji"], "🎉");
    assert_eq!(parsed["label"], "celebrate 🎊 now");
    assert_eq!(state.status, AnthropicStatus::MessageStop);
}

#[test]
fn anthropic_ping_mid_stream_is_silent_no_op() {
    let bytes = read_fixture("anthropic_ping_mid_stream.sse");
    let mut state = AnthropicStreamState::new();
    for event in drain_events(&bytes) {
        state.apply(event).expect("apply must not error");
    }

    let block = state.blocks.get(&0).expect("block 0 present");
    assert_eq!(block.text_buffer, "Hello, world.");
    assert_eq!(state.status, AnthropicStatus::MessageStop);
}

#[test]
fn anthropic_error_mid_stream_sets_errored_and_preserves_partial_content() {
    let bytes = read_fixture("anthropic_error_mid_stream.sse");
    let mut state = AnthropicStreamState::new();
    for event in drain_events(&bytes) {
        state.apply(event).expect("apply must not error");
    }

    assert_eq!(state.status, AnthropicStatus::Errored);
    let block = state.blocks.get(&0).expect("block 0 present");
    assert_eq!(block.text_buffer, "Partial respo");
    assert!(
        !block.complete,
        "no content_block_stop arrived before the error"
    );
}

#[test]
fn anthropic_tcp_drop_before_message_stop_leaves_status_open() {
    let bytes = read_fixture("anthropic_tcp_drop_before_message_stop.sse");
    let mut state = AnthropicStreamState::new();
    for event in drain_events(&bytes) {
        state.apply(event).expect("apply must not error");
    }

    let block = state.blocks.get(&0).expect("block 0 present");
    assert_eq!(block.text_buffer, "This response got cut off");
    assert!(block.complete);
    assert_eq!(state.stop_reason.as_deref(), Some("end_turn"));
    // The state machine has no way to distinguish "still streaming"
    // from "connection silently dropped" -- status stays Open. The
    // proxy layer must detect the dropped connection independently
    // (this test documents that contract, not a bug).
    assert_eq!(state.status, AnthropicStatus::Open);
}

#[test]
fn openai_chat_tool_call_id_and_name_only_on_first_chunk() {
    let bytes = read_fixture("openai_chat_tool_call_split.sse");
    let mut state = ChunkState::new();
    for event in drain_events(&bytes) {
        state.apply(event).expect("apply must not error");
    }

    let choice = state.choices.get(&0).expect("choice 0 present");
    let tool_call = choice.tool_calls.get(&0).expect("tool call 0 present");
    assert_eq!(tool_call.id.as_deref(), Some("call_01"));
    assert_eq!(tool_call.call_type.as_deref(), Some("function"));
    assert_eq!(tool_call.function_name.as_deref(), Some("get_weather"));
    assert_eq!(
        tool_call.function_arguments,
        "{\"location\":\"San Francisco\"}"
    );
    assert_eq!(choice.finish_reason.as_deref(), Some("tool_calls"));
    assert_eq!(state.status, OpenAiChatStatus::Done);
}

#[test]
fn openai_chat_done_sentinel_tolerates_trailing_whitespace() {
    let bytes = read_fixture("openai_chat_done_with_trailing_whitespace.sse");
    let mut framer = SseFramer::new();
    framer.push(&bytes);

    let mut state = ChunkState::new();
    while let Some(result) = framer.next_event() {
        let event = result.expect("fixture must be well-formed SSE");
        state.apply(event).expect("apply must not error");
    }

    let choice = state.choices.get(&0).expect("choice 0 present");
    assert_eq!(choice.content, "Hi there.");
    assert_eq!(choice.finish_reason.as_deref(), Some("stop"));
    assert_eq!(state.status, OpenAiChatStatus::Done);
    assert!(framer.done_seen());

    // Whatever trailing whitespace/blank-block bytes followed [DONE],
    // draining must not error and must not lose track of state --
    // it's fine for leftover non-terminated bytes to remain buffered.
    assert!(framer.next_event().is_none());
}

#[test]
fn openai_responses_out_of_order_item_completion_by_id() {
    let bytes = read_fixture("openai_responses_out_of_order_done.sse");
    let mut state = ResponseState::new();
    for event in drain_events(&bytes) {
        state.apply(event).expect("apply must not error");
    }

    // item_fn (added second) completes BEFORE item_msg (added first)
    // on the wire -- P1-17. Both must resolve correctly keyed by id.
    let fn_item = state.items.get("item_fn").expect("item_fn present");
    assert!(fn_item.complete);
    assert_eq!(fn_item.function_call_arguments, "{\"q\":\"weather\"}");

    let msg_item = state.items.get("item_msg").expect("item_msg present");
    assert!(msg_item.complete);
    assert_eq!(msg_item.output_text, "Looking that up.");

    assert_eq!(state.status, ResponsesStatus::Completed);
    assert_eq!(state.service_tier.as_deref(), Some("default"));
}

#[test]
fn openai_429_application_json_body_is_not_misparsed_as_sse() {
    // A rate-limit error returned before any streaming begins comes
    // back as a normal `application/json` body, not an SSE stream.
    // The framer must not misinterpret it: there's no `data:` line
    // prefix in a raw JSON error body, so nothing should ever be
    // yielded as an event. This documents the real production
    // contract: callers must branch on Content-Type BEFORE handing
    // bytes to the SSE framer, not rely on the framer to sniff it.
    let raw = read_fixture("openai_429_as_application_json.http");

    // Split header block (terminated by the first blank line) from
    // the JSON body, mirroring how an HTTP client would already have
    // separated them before any SSE-specific code ran.
    let sep = raw
        .windows(2)
        .position(|w| w == b"\n\n")
        .expect("fixture must contain a header/body separator");
    let body = &raw[sep + 2..];
    let body_str = std::str::from_utf8(body).expect("body must be UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(body_str).expect("body must be valid JSON");
    assert_eq!(parsed["error"]["type"], "rate_limit_error");

    let mut framer = SseFramer::new();
    framer.push(body);
    // No blank-line terminator inside the JSON body, so the framer
    // has nothing to yield -- it should not panic, error, or invent
    // an event out of unterminated bytes.
    assert!(framer.next_event().is_none());
    assert_eq!(framer.buffered_len(), body.len());
    assert_eq!(framer.take_remaining(), Bytes::from(body.to_vec()));
}
