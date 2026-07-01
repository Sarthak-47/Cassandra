# Cassandra

**The Context Optimization Layer for LLM Applications.**

Compress everything your AI agent reads — tool outputs, DB queries, file
reads, RAG results, logs — before it hits the model. Same answers, a
fraction of the tokens.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

## What it does

Every tool call, DB query, file read, and RAG retrieval your agent makes is
70-95% boilerplate. Cassandra compresses it away before it reaches the LLM.
The model sees less noise, responds faster, and costs less.

```
Your Agent / App
    |
    |  tool outputs, logs, DB reads, RAG results, file reads, API responses
    v
 Cassandra   <- proxy, Python library, or framework integration
    |
    v
 LLM Provider  (OpenAI, Anthropic, Google, Bedrock, 100+ via LiteLLM)
```

Cassandra works as a transparent proxy (zero code changes), a Python
function (`compress()`), or a framework integration (LangChain, Agno,
Strands, LiteLLM, MCP).

## Quick start

**Proxy (zero code changes):**

```bash
pip install "cassandra-ai[all]"
cassandra proxy
```

```bash
# Point any tool at the proxy
ANTHROPIC_BASE_URL=http://localhost:8787 claude
OPENAI_BASE_URL=http://localhost:8787/v1 your-app
```

**Python SDK:**

```python
from cassandra import compress

result = compress(messages, model="claude-sonnet-4-5-20250929")
response = client.messages.create(
    model="claude-sonnet-4-5-20250929",
    messages=result.messages,
)
print(f"Saved {result.tokens_saved} tokens ({result.compression_ratio:.0%})")
```

**Coding agents:**

```bash
cassandra wrap claude       # Claude Code
cassandra wrap codex        # OpenAI Codex CLI
cassandra wrap aider        # Aider
cassandra wrap cursor       # Cursor
```

See the [wiki](wiki/index.md) for the full quickstart, integration guide,
and architecture deep dive.

## How it works

Cassandra runs a two-stage pipeline on every request:

1. **CacheAligner** stabilizes message prefixes so the provider's prompt
   cache actually hits.
2. **ContentRouter** auto-detects content type (JSON, code, logs, search
   results, diffs, HTML, plain text) and routes each to the optimal
   compressor: SmartCrusher, CodeCompressor, Kompress, LogCompressor,
   SearchCompressor, DiffCompressor, or HTMLExtractor.

Compressed content is never discarded — it goes into the CCR
(Compress-Cache-Retrieve) store, and the model gets a `cassandra_retrieve`
tool to fetch the full original when it needs more detail.

See [wiki/ARCHITECTURE.md](wiki/ARCHITECTURE.md) for the full pipeline
design.

## Building from source

This is a mixed Rust/Python workspace built with
[maturin](https://www.maturin.rs/):

```bash
python -m pip install maturin
pip install -e .[dev]
```

`pyproject.toml` builds the `cassandra._core` extension from
[crates/cassandra-py](crates/cassandra-py), which wraps
[crates/cassandra-core](crates/cassandra-core) (compression engine) and
depends on the Rust workspace also containing
[crates/cassandra-proxy](crates/cassandra-proxy) (the proxy server) and
[crates/cassandra-parity](crates/cassandra-parity) (Rust-vs-Python parity
harness).

```bash
cargo check --workspace   # build the Rust workspace
pytest                    # run the Python test suite
```

## License

Apache 2.0 — free for commercial use.
