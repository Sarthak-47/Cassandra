//! Phase C PR-C4: Conversations API awareness.
//!
//! OpenAI's Conversations API (`conversation: {"id": "conv_..."}` on a
//! `/v1/responses` request) means the server-side conversation object
//! -- not the request body -- is the source of truth for prior turns.
//! The live-zone dispatcher's compression decisions (frozen-prefix
//! detection, tool/schema normalization, cache-hot-zone hashing) all
//! assume the request body is the complete conversation; with a
//! `conversation.id` present, the local view the proxy sees may be
//! incomplete or entirely absent (the client can send just the new
//! turn and let the server thread it onto the stored conversation).
//!
//! Per REALIGNMENT/05-phase-C-rust-proxy.md PR-C4: rather than guess
//! at a correct compression decision against an incomplete view,
//! disable live-zone compression entirely for these requests until a
//! cross-request shared cache (Phase 4 of the product roadmap) can
//! reconstruct the full conversation server-side. This module is the
//! detector only -- callers (the `/v1/responses` live-zone dispatcher)
//! decide what to do with the result.

use serde_json::Value;

/// Extract the `conversation.id` field from a parsed `/v1/responses`
/// request body, if present.
///
/// Returns `None` for any other shape: `conversation` absent, present
/// but not an object, `id` absent, or `id` present but not a string.
/// None of those are errors -- they just mean "not a Conversations API
/// request", which is the overwhelmingly common case.
pub fn conversation_id(body: &Value) -> Option<&str> {
    body.get("conversation")?.get("id")?.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_conversation_id_when_present() {
        let body = json!({
            "model": "gpt-5.1",
            "conversation": {"id": "conv_abc123"},
            "input": "hello",
        });
        assert_eq!(conversation_id(&body), Some("conv_abc123"));
    }

    #[test]
    fn returns_none_when_conversation_absent() {
        let body = json!({"model": "gpt-5.1", "input": "hello"});
        assert_eq!(conversation_id(&body), None);
    }

    #[test]
    fn returns_none_when_conversation_is_not_an_object() {
        let body = json!({"model": "gpt-5.1", "conversation": "conv_abc123"});
        assert_eq!(conversation_id(&body), None);
    }

    #[test]
    fn returns_none_when_id_absent() {
        let body = json!({"model": "gpt-5.1", "conversation": {}});
        assert_eq!(conversation_id(&body), None);
    }

    #[test]
    fn returns_none_when_id_is_not_a_string() {
        let body = json!({"model": "gpt-5.1", "conversation": {"id": 12345}});
        assert_eq!(conversation_id(&body), None);
    }
}
