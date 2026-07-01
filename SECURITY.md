# Security Policy

Cassandra runs as a proxy in front of LLM provider traffic and handles
API keys, provider credentials, and (depending on configuration)
customer request/response content. Treat anything that could leak
credentials, bypass auth-mode gating, or exfiltrate compressed content
as a security issue, not just a bug.

## Supported Versions

This project is pre-1.0 and under active development. Only the latest
release on `main` is supported — there is no backport policy yet.

## Reporting a Vulnerability

**Do not open a public GitHub issue for a security vulnerability.**

Report privately via
[GitHub Security Advisories](https://github.com/Sarthak-47/cassandra/security/advisories/new)
for this repository. This creates a private discussion with maintainers
before any public disclosure.

Include:

- A description of the vulnerability and its potential impact.
- Steps to reproduce (a minimal repro is ideal — a proxy request/response
  pair, a config, or a code snippet).
- Which component is affected (Python `cassandra/`, Rust
  `crates/cassandra-*`, a specific integration, the wrap CLIs, etc.).

## What We Consider In Scope

- Credential/API-key leakage (logs, error messages, upstream headers,
  `X-Cassandra-*` header handling).
- Prompt-cache-safety violations that could cause cross-request data
  bleed.
- Auth-mode misclassification (PAYG/OAuth/subscription) that changes
  which compression policy or headers a request gets.
- Compression logic that could cause data loss or corruption in ways
  that change model behavior unsafely (not just accuracy regressions —
  file a normal issue for those).
- Dependency vulnerabilities in the resolved dependency tree (Rust via
  `cargo audit`, Python via `pip-audit` against `uv.lock`).

## Response

We aim to acknowledge new reports within a few days. Fix timeline
depends on severity — critical credential-leak or cache-safety issues
get prioritized over everything else in the project, including the
[REALIGNMENT](REALIGNMENT/INDEX.md) rewrite in progress.
