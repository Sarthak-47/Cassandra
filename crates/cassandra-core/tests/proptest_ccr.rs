//! REALIGNMENT Phase I PR-I3 — CCR store round-trip property test.
//!
//! Per `REALIGNMENT/11-phase-I-test-infra.md`: "round-trip property
//! test: `decompress(compress(content)) == content` for any content."
//!
//! CCR (Compress-Cache-Retrieve) doesn't itself compress bytes on the
//! wire -- the live-zone dispatcher's compressors (SmartCrusher etc.)
//! do that. CCR's job is storing the *original* content under a
//! content-addressed hash so `cassandra_retrieve(hash)` can recover it
//! later. So the property under test here is the store's own
//! contract: `get(put(content).hash) == Some(content)` for any string
//! content, using the same `compute_key` hashing every real call site
//! uses (live-zone dispatcher, tool-retrieval endpoint, Python parity).
//!
//! Runs against `InMemoryCcrStore` -- fast, no filesystem/network,
//! exercises the same `CcrStore` trait contract `SqliteCcrStore` and
//! `RedisCcrStore` implement (see PR-B7's `CcrStore` trait in
//! `cassandra_core::ccr`).

use cassandra_core::ccr::{compute_key, CcrStore, InMemoryCcrStore};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

    /// The core round-trip: whatever you put in under its own content
    /// hash, you get back byte-for-byte.
    #[test]
    fn ccr_round_trip_get_returns_exact_put(content in ".*") {
        let store = InMemoryCcrStore::new();
        let hash = compute_key(content.as_bytes());
        store.put(&hash, &content);
        prop_assert_eq!(store.get(&hash), Some(content));
    }

    /// Re-storing the same content under its own hash is idempotent --
    /// putting twice must not change what comes back (the trait's own
    /// doc comment states this contract explicitly).
    #[test]
    fn ccr_re_put_same_content_is_idempotent(content in ".*") {
        let store = InMemoryCcrStore::new();
        let hash = compute_key(content.as_bytes());
        store.put(&hash, &content);
        store.put(&hash, &content);
        prop_assert_eq!(store.get(&hash), Some(content));
        prop_assert_eq!(store.len(), 1);
    }

    /// A hash that was never put() returns None -- the store must
    /// never fabricate content for an unknown key.
    #[test]
    fn ccr_unknown_hash_returns_none(content in ".+") {
        let store = InMemoryCcrStore::new();
        // `content` is only used to derive a hash that (with
        // overwhelming probability) was never stored -- store stays
        // empty throughout.
        let hash = compute_key(content.as_bytes());
        prop_assert_eq!(store.get(&hash), None);
    }

    /// `compute_key` is a pure function: same payload, same key, every
    /// call. Downstream code (the live-zone dispatcher, the retrieval
    /// endpoint) relies on this to find what it stored.
    #[test]
    fn compute_key_is_deterministic(content in ".*") {
        let a = compute_key(content.as_bytes());
        let b = compute_key(content.as_bytes());
        prop_assert_eq!(a, b);
    }
}
