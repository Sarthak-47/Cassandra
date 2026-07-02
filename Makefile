.PHONY: test-parity test-byte-faithful test-cache-hot-zone test-tool-def-stability ci-precheck build-wheel lint fmt test test-rust test-python

# Rust-vs-Python parity harness (crates/cassandra-parity). Compares the two
# implementations against recorded fixtures under tests/parity/fixtures;
# `Skipped` comparators are fine (not yet promoted from stub), any `Diff`
# fails the build. See REALIGNMENT/11-phase-I-test-infra.md PR-I6.
test-parity:
	cargo run -p cassandra-parity --bin parity-run -- run --fixtures tests/parity/fixtures

# SHA-256 byte-faithful round-trip gate (REALIGNMENT/11-phase-I-test-infra.md
# PR-I1) -- "the single most important regression test for cache safety."
# Runs quickly (<5s); safe as a per-PR CI gate independent of the full suite.
test-byte-faithful:
	cargo test -p cassandra-proxy --test integration_byte_faithful
	pytest tests/test_proxy_byte_faithful_forwarding.py -k byte_equal_sha256

# Cache hot zone non-mutation gate (REALIGNMENT/11-phase-I-test-infra.md
# PR-I7) -- proves system/tools/frozen-messages/thinking-signatures/
# opaque Responses items survive byte-identical even while live-zone
# compression is actively shrinking the rest of the same request.
test-cache-hot-zone:
	cargo test -p cassandra-proxy --test integration_cache_hot_zone

# Tool-definition byte-stability golden-file gate
# (REALIGNMENT/11-phase-I-test-infra.md PR-I8) -- pins the canonical
# bytes of every auto-injected tool definition (cassandra_retrieve,
# memory_save, memory_search) so a schema-shape or wording change fails
# CI loudly instead of silently busting every active session's prompt
# cache. Python-owned for now (see the module docstring in
# tests/test_tool_def_byte_stability.py for why this isn't a Rust
# test); revisit once Phase H ports tool-definition construction there.
test-tool-def-stability:
	pytest tests/test_tool_def_byte_stability.py

# Pre-push gate. Fast, local-only checks -- run this before every push to
# main (per REALIGNMENT/INDEX.md convention). Does not run the full pytest
# suite (too slow for a pre-push habit); CI runs that.
ci-precheck: fmt lint
	cargo check --workspace --all-targets
	cargo test --workspace --lib
	ruff check .
	ruff format --check .
	mypy cassandra --ignore-missing-imports

build-wheel:
	maturin build --profile ci --out dist

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test: test-rust test-python

test-rust:
	cargo test --workspace

test-python:
	pytest tests/
