.PHONY: test-parity ci-precheck build-wheel lint fmt test test-rust test-python

# Rust-vs-Python parity harness (crates/cassandra-parity). Compares the two
# implementations against recorded fixtures under tests/parity/fixtures;
# `Skipped` comparators are fine (not yet promoted from stub), any `Diff`
# fails the build. See REALIGNMENT/11-phase-I-test-infra.md PR-I6.
test-parity:
	cargo run -p cassandra-parity --bin parity-run -- run --fixtures tests/parity/fixtures

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
