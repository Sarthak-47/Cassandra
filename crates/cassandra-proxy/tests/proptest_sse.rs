//! REALIGNMENT Phase I PR-I2 — SSE parser fuzz test.
//!
//! Per `REALIGNMENT/11-phase-I-test-infra.md`: "Property test runs 10K
//! random byte sequences without panic." The framer's whole reason for
//! existing (see `crates/cassandra-proxy/src/sse/framing.rs`'s module
//! doc) is that the old Python proxy's `errors="ignore"` UTF-8 decode
//! silently corrupted split-codepoint chunks in production -- the
//! Rust framer's contract is "never panic, surface malformed input as
//! a recoverable `Result`, not a crash." This test is the direct proof
//! of that contract: arbitrary bytes (not just "arbitrary bytes that
//! happen to look like SSE") must never panic the framer, regardless
//! of how many chunks they're split across or where the splits land
//! relative to UTF-8 codepoint boundaries.

use cassandra_proxy::sse::SseFramer;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 10_000, ..ProptestConfig::default() })]

    /// Arbitrary bytes fed to the framer in one shot must never panic.
    /// Every `next_event()` result (framing error or a real event) is
    /// drained; we don't assert anything about the *content* -- random
    /// bytes have no expected shape -- only that draining never panics.
    #[test]
    fn sse_parser_no_panic_on_arbitrary_bytes(chunk in prop::collection::vec(any::<u8>(), 0..2048)) {
        let mut framer = SseFramer::new();
        framer.push(&chunk);
        while let Some(_result) = framer.next_event() {
            // Draining is the point: `next_event()` must return, not panic,
            // for every framing state arbitrary bytes can put it in.
        }
    }

    /// Same property, but the bytes arrive split across multiple
    /// `push()` calls at arbitrary boundaries -- including boundaries
    /// that land mid-UTF-8-codepoint, which is exactly the class of
    /// input that broke the Python proxy (P1-15, see the framer's
    /// module doc). The framer must buffer correctly across calls and
    /// never panic regardless of where a chunk boundary falls.
    #[test]
    fn sse_parser_no_panic_on_arbitrary_chunked_bytes(
        chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..256), 0..16)
    ) {
        let mut framer = SseFramer::new();
        for chunk in &chunks {
            framer.push(chunk);
            while let Some(_result) = framer.next_event() {
                // Same as above: draining must not panic after every push.
            }
        }
    }
}
