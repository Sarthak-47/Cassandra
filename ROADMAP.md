# Cassandra Roadmap

Last updated: 2026-07-02

This is a living document tracking what's done, what's broken, and what's
next for Cassandra. It complements [REALIGNMENT/](REALIGNMENT/INDEX.md),
which covers the compression-engine rewrite in detail — this file is the
higher-level view across infra, governance, releases, and product.

---

## 1. CI / infra gaps

Same root cause across all of these: config files CI assumes exist but were
never committed to the repo. Small, mechanical fixes once picked up.

**All 10 top-level workflows on `main` are green as of 2026-07-02** (CI,
Docker, Security, Release Please, Dev Containers, Init/Wrap/Install E2E ×
native and Docker variants). Nothing outstanding in this section right now.

### Done (2026-07-02)
- [x] Root `Dockerfile` + `docker-bake.hcl` — 8-variant matrix
      (runtime/runtime-nonroot/-code/-code-nonroot/-slim/-slim-nonroot/
      -code-slim/-code-slim-nonroot), all built and pushed successfully.
- [x] `.release-please-config.json` / `.release-please-manifest.json` —
      root package now declares `release-type: python`, `package-name`,
      and `extra-files` for the two npm package.json's that must stay in
      lockstep.
- [x] Branch protection on `main` requiring the `CI` check.
- [x] `ast-grep-cli`, `sqlite-vec`, `sentence-transformers` declared as
      real dependencies (their absence was silently masked by other bugs
      until the full test suite was run end-to-end — see below).
- [x] `litellm` and `sentence-transformers` deps carry a
      `python_version < '3.14'` marker (GH #956) so `pip install
      cassandra-ai` stays satisfiable on 3.14.
- [x] `test` job in `ci.yml` now sets up a Rust toolchain — several
      `test_release_workflows.py` assertions shell out to `cargo tree`
      and silently failed without one.
- [x] LICENSE/NOTICE listed in `[tool.maturin].include` for sdist format
      (PEP 639 auto-discovery requires this or PyPI rejects the upload).
- [x] Root `Cargo.toml` workspace + `pyproject.toml` — fixed `build` /
      `build-wheel` CI jobs (maturin: "could not find Cargo.toml").
- [x] `rust-toolchain.toml`, `README.md`, `LICENSE`, `uv.lock` — all
      referenced by CI/Dockerfiles but never existed.
- [x] `[profile.ci]` in `Cargo.toml` — `build-wheel`'s
      `maturin build --profile ci` had nothing to resolve.
- [x] `scripts/validate-workflows.sh` — `workflow-validation` job's last
      step didn't exist (exit 127).
- [x] `mkdocs.yml` — the "Deploy Documentation" workflow's `mkdocs build`
      had no config; `wiki/` is already written in mkdocs-material syntax.
- [x] `scripts/ci/verify_hf_model_cache.py` — `test` job's HF cache
      verification step didn't exist.
- [x] 433 mypy errors in the proxy handler mixins, resolved to 0.
- [x] 5 real compile errors in `crates/cassandra-proxy` (axum 0.8 API
      breakage).
- [x] `ruff format` / import-sort pass across the whole repo.
- [x] `[agno]` / `[relevance]` extras in `pyproject.toml` completed
      (missing `openai`, `tiktoken`, `fastembed`).

---

## 2. Known limitations (not blocking, just worth knowing)

- The `litellm` CVE fix (bumped floor to `>=1.84.0`) only covers Python
  3.10–3.13 — litellm's patched releases don't support Python 3.14 yet.
  Marked with `python_version < '3.14'` so installs stay satisfiable;
  nothing further to do until litellm ships 3.14 support upstream.
- `[ml]` extras (torch/transformers/sentence-transformers) were never
  pip-audited — the full install was too heavy to complete in one session.
  Worth a follow-up audit pass.
- `mkdocs.yml`'s nav intentionally excludes `wiki/plans/*` (internal
  planning docs) — those pages exist and build, just aren't in the menu.

---

## 3. Governance docs

- [x] `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` — all added.

---

## 4. Release readiness — unverified, real risk

- [ ] **Confirm `cassandra-ai` is actually available on PyPI and npm.**
      Docs and `pyproject.toml` both assume this name is claimable — never
      independently verified. Check before the `Publish` workflow's first
      real run.
- [ ] Decide the `LICENSE` copyright line (currently "Sarthak-47 and
      Cassandra Contributors" — a placeholder I picked, change if you want
      something else).

---

## 5. Dependabot

Currently **disabled** (`.github/dependabot.yml` removed, config preserved
in git history) — paused deliberately until the project is further along.
Re-enable when ready; the 5-ecosystem weekly config (Docker, GitHub
Actions, pip, cargo, npm) is a one-command restore from history.

---

## 6. The compression-engine rewrite (REALIGNMENT) — ~76% implemented, but see the critical caveat below

**CRITICAL FINDING (2026-07-02):** the standalone Rust proxy
(`crates/cassandra-proxy`) that Phases C and D's work lives in is **not
what `cassandra proxy` / `cassandra wrap` actually start**. Verified
directly: `cassandra/cli/proxy.py`'s `proxy` command does `from
cassandra.proxy.server import ...` — the real, deployed traffic path is
still the Python FastAPI server (`cassandra/proxy/server.py`), not the
Rust binary. The planned cutover mechanism
(`CASSANDRA_PROXY_BACKEND={python|rust}`, recommended for Phase H PR-H1
in [12-decisions-needed.md](REALIGNMENT/12-decisions-needed.md)) **does
not exist anywhere in the codebase** — grepped, zero hits outside that
planning doc. A comment in `crates/cassandra-py/src/lib.rs:1560-1569`
confirms this was a known, deliberate hot-fix: a PyO3 binding was added
to restore compression on the Python side specifically *because* "that
binary is not deployed by the CLI."

**What this means:** Phases C and D's Rust code is genuinely
well-built and well-tested (verified directly against spec, see below)
— but it's currently unreachable dead weight from a production-usage
standpoint. The "76% implemented" figure measures code that exists, not
code that's live for a `cassandra proxy` user today.

**Correction to an earlier draft of this section:** I initially wrote
that Phase H PR-H1 (a `CASSANDRA_PROXY_BACKEND` switch) was the
highest-leverage next step. That undersold what PR-H1 actually is —
read [10-phase-H-python-retirement.md](REALIGNMENT/10-phase-H-python-retirement.md)
directly: it's not a switch, it's an outright **-15,000/+500 LOC,
explicitly HIGH-RISK deletion** of the entire Python proxy (server,
handlers, memory subsystem, semantic cache, batch handler — the works),
with acceptance criteria including a canary deploy, a 24h staging
soak with cache-hit-rate parity, and a mandated 30-day rollback image
retention. Its own prerequisites list **"Real-traffic shadow test
(Phase I) shows Rust ≥99.9% byte-equality vs Python"** — and Phase I
has zero markers found in code (see below). **PR-H1 cannot responsibly
be started before Phase I's shadow-testing infra exists**, and even
once it can, this is exactly the kind of production-facing,
hard-to-reverse change that needs explicit human sign-off and
operations coordination, not autonomous execution. The actual
highest-leverage next step is **Phase I** (shadow/canary test infra),
which is the real gate — not H1 itself.

**Correction (2026-07-02):** the original "0% started" claim below was
wrong. It was based only on the absence of `realign-*` git branches/commits,
not on what the code actually contains. Grepping the tracked `PR-<phase><n>`
markers that REALIGNMENT's own docs use to label each unit of work against
what's actually referenced in `crates/` and `cassandra/` source comments:
**39 of the 51 planned PRs have a corresponding implementation marker in
code.** Phases A–G are each fully represented at the marker level (every
`PR-A*`...`PR-G*` ID shows up somewhere in source); the gap is almost
entirely Phase H (Python proxy retirement — only `PR-H2` found, `H1/H3/H4`
missing) and Phase I (test-infra — only `PR-I10` found as of this session's
work, see below).

**Update: every phase A–G has now been read directly against its spec**
(not just marker-grep) — see the per-phase detail below. Marker presence
turned out to be a poor completeness proxy: A and D hold up as genuinely
solid, G is nearly solid, B/C/E have real-but-survivable gaps, and F has
confirmed security-relevant gaps that mean it should not be treated as
done. Don't trust the "Implemented (N/N markers)" framing alone for any
phase without reading its detail note.

Full detail in [REALIGNMENT/](REALIGNMENT/INDEX.md).

| Phase | What | Status |
|---|---|---|
| A — Lockdown | Stop the cache-busting bugs (passthrough on `/v1/messages`) | **Verified** genuinely done (all 8, one spec-stale note — see below) |
| B — Live-zone engine | Delete ~10K LOC (ICM/scoring/relevance), rebuild compression | **Verified** mostly done — CodeCompressor gap fixed 2026-07-02 (heuristic MVP, not a tree-sitter port); relevance/ deletion investigated and correctly NOT done (it's live code) — see below |
| C — Rust proxy paths | Port remaining handlers, byte-level SSE parser | **Verified** well-built but not actually deployed (see critical finding above) — Conversations-API-awareness gap fixed 2026-07-02 |
| D — Bedrock/Vertex native | Replace the currently-fake LiteLLM conversion | **Verified** genuinely native (SigV4/EventStream/ADC, real, not a LiteLLM shim) |
| E — Cache stabilization | Deterministic tool/schema ordering | **Verified** mostly solid, 1/6 has a smaller acceptance-criteria gap (E2, E5, E6 fixed 2026-07-02, see below) |
| F — Auth-mode policy | PAYG/OAuth/subscription-aware compression | **Verified, fully closed** — every gap found (F2 accept-encoding, F3 raw token + TOIN keying, F4) fixed (2026-07-02) — see below |
| G — RTK + observability | Broader wrap-CLI support, metrics | **Verified** strongest phase audited, 1 minor gap; unblocks Phase I PR-I9 |
| H — Python retirement | Delete the Python proxy once Rust hits parity | Mostly not started (1/4 — only PR-H2); PR-H1 is a HIGH-RISK -15K LOC deletion, not startable yet (see below) |
| I — Test infra | SHA-256 round-trip tests, parity gates | 7/10 landed (I1, I2, I3, I7, I8, I9, I10 — all verified green in real CI); I5/I6 blocked on B3's CodeCompressor gap, I4 needs Phase A-G fully verified first (see below) |

**Phase A spot-check result (2026-07-02):** all 8 PR-A markers verified
against actual code, not just grep — read the real implementation of
each (`cache_control.rs`, `helpers.py`, `streaming.py`, `headers.rs`,
`cache_aligner.py`, `SessionBetaTracker`, etc.) and confirmed against
[REALIGNMENT/03-phase-A-lockdown.md](REALIGNMENT/03-phase-A-lockdown.md).
PR-A1–A7: genuinely done, real logic + real tests, no stubs/no-ops/TODOs.
Two notes there: (1) several implementations live in different
files/functions than the spec names — drift in location, not substance;
(2) PR-A1's "pure passthrough stub" has already been superseded by live
Phase B compression code gated behind `CompressionMode` — the codebase
is ahead of Phase A's literal spec text, not behind it.

**PR-A8 note (resolved, not a gap):** the spec calls for two fixes in
`cassandra/proxy/responses_converter.py` (preserve the `phase` field,
add an unknown-item-type warning — see
[03-phase-A-lockdown.md:350-362](REALIGNMENT/03-phase-A-lockdown.md)),
but that file doesn't exist. Traced it: `crates/cassandra-proxy/src/
proxy.rs`'s `compress_openai_responses_request` (PR-C3) now handles
`/v1/responses` natively in Rust — Phase C ported this logic off Python
entirely, so the file's removal is a later phase superseding an earlier
one, not a dropped requirement. The spec section is stale, not the code.
The other three PR-A8 sub-items (SSE delta arms, bytes-buffer decode
replacing `errors="ignore"`, Rust 413-vs-400 + upstream request-id
capture) are genuinely implemented.

**Phase B spot-check result:** 5 of 7 PRs (B2, B4, B5, B6, B7) are solid,
real implementations with genuine tests. Two real gaps found; one is
**not actually fixable as originally framed, one is fixed:**

(1) **PR-B1** — investigated (2026-07-02) and determined NOT to be a
safe fix: `crates/cassandra-core/src/relevance/` (~1,300 LOC of
BM25/embedding/hybrid scorers) is genuinely, actively imported by 5
real files across `smart_crusher/` and `text_crusher/` — it's
load-bearing for the current B2+ replacement architecture's
relevance-aware compression, not leftover dead code. The original
spec assumed it would become fully unreachable after the ICM/scoring
deletion; the codebase correctly evolved to keep it because
SmartCrusher/TextCrusher still need it. Deleting it per the literal
spec would be a regression, not a fix — the spec is stale here, not
the code (same pattern as the PR-A8 note above). Left as-is; the
`fastembed` ML dependency stays live for the same reason.

(2) **PR-B3's CodeCompressor gap is fixed (2026-07-02).**
`ContentType::SourceCode` was hard-coded to `NoOp` with an explicit
`TODO(PR-B4 / Rust code-compressor port)` comment — source-code
tool_results got zero compression in production. Deliberately did
NOT port Python's `CodeAwareCompressor` (tree-sitter AST parsing
across 7 languages with semantic function-importance ranking, ~2,100
LOC, plus a heavy optional dependency even Python's own fallback
path doesn't avoid — it delegates to Kompress, which is *also*
unwired on the Rust side). Instead built
`crates/cassandra-core/src/transforms/code_compressor.rs`, a
deliberately scoped MVP: pure line-based heuristics (no AST, no new
dependencies) that classifies every line as Signature
(declaration/scope-opener — never touched, the load-bearing safety
property), Comment (start-of-line marker only, stripped), or Body
(truncated in long contiguous runs, keeping head/tail with an
omission marker). Not syntax-validity-guaranteeing or semantic,
unlike Python's promise — an explicit, documented tradeoff given the
read-by-an-LLM-not-executed context. Wired into `live_zone.rs`'s
existing dispatcher via the same pattern as its siblings
(SmartCrusher/LogCompressor/DiffCompressor); PR-B4's byte-threshold
and tokenizer-validated-fallback safety nets wrap it automatically,
unchanged. 13 new unit tests (including a real bug caught during
testing: a standalone closing brace `}` was being swept into
body-run truncation and could push out real content — fixed by
treating bare scope-closers as signature-like). Replaced the old
no-op-pinning integration test with
`source_code_tool_result_routes_to_code_compressor`. Verified green
on real CI (`rust.yml`'s `test (ubuntu)` job — full workspace
`cargo test`, `cargo fmt --check`, `cargo clippy` — and the full
`ci.yml` suite; first attempt hit a transient CI-infra disk-space
failure on the runner, unrelated to the code, confirmed by fmt/clippy
having already passed cleanly before it; a rerun came back green).

**Phase C spot-check result:** C1–C3 are solid (byte-level SSE parser,
`/v1/chat/completions`, and an unusually thorough `/v1/responses` item-type
handler in Rust). Two real gaps found; one fixed: **(1) PR-C4's
Conversations-API-awareness gap is fixed (2026-07-02).** Added
`crates/cassandra-proxy/src/conversations.rs` (a pure
`conversation_id()` detector — what existed under a similar name,
`handlers/conversations.rs`, is the unrelated `/v1/conversations*`
CRUD passthrough). Wired into `live_zone_responses.rs` *before* the
`has_array_field` gate (a Conversations API request may legitimately
omit `input`/`messages` entirely), logging the spec-required INFO
`compression_decision` event with the real `conversation_id` and
incrementing a new unlabelled `proxy_conversations_api_request_count_total`
counter, then returning `Outcome::Passthrough` — the existing
byte-length cache-safety alarm covers non-mutation for free. New test
landed in the existing `tests/integration_conversations.rs` (reusing
its helpers) proves a 4096-byte `function_call_output` survives
uncompressed. Verified green on real CI (`rust.yml`'s `test (ubuntu)`
job and the full `ci.yml` suite). **(2) PR-C5**'s headline claim —
"after this PR lands, no Python code is on the `/v1/responses` request
path" — is the critical finding above: falsified by the codebase's own
comments (`crates/cassandra-py/src/lib.rs:1560-1569`), since the Rust
proxy binary isn't what `cassandra proxy` deploys.

**Phase E spot-check result:** substantially real work, no dead-code
stubs, genuine wiring into the hot path — but 4 of 6 PRs fell short of
their own acceptance criteria in ways worth knowing about, not
blockers: **E2's missing snapshot test is fixed (2026-07-02).** Added
`snapshot_real_production_schema_memory_save` to
`crates/cassandra-proxy/src/cache_stabilization/tool_def_normalize.rs`,
sourcing the real `memory_save` tool's `input_schema` from the same
golden file PR-I8 already pinned
(`tests/golden/tool_defs/memory_save_anthropic.json`) via
`include_str!`, so the two can't silently drift apart. Exercises real
structure the prior synthetic-only tests didn't: multiple sibling
properties genuinely out of alphabetic order, array-of-objects
properties whose nested `properties` also need recursive sorting, and
two `required` arrays whose element order must survive untouched even
though a sibling `properties` object gets reordered. Verified green
on real CI (`rust.yml`'s `test (ubuntu)` job and the full `ci.yml`
suite). **E3** (Anthropic `cache_control`
auto-placement) ships exactly 1 marker (last tool only) instead of the
spec's 4 (system/tools/history-boundary/latest-user) — a deliberate,
well-documented first-ship scope cut pending telemetry, not a bug.
**E5's pattern-coverage gap is fixed (2026-07-02).** Added
`VolatileKind::Jwt` (three dot-separated base64url segments anchored
on the `eyJ` header prefix) and `VolatileKind::BuildHash` (contiguous
runs of >= 32 hex digits) to
`crates/cassandra-proxy/src/cache_stabilization/volatile_detector.rs`,
following the existing no-regex byte-position-walk pattern the
timestamp/UUID matchers already use. All 5 spec'd pattern types now
detected. 6 new unit tests (JWT detection + false-positive rejection,
sha1/sha256-length build-hash detection, sub-32-char and
hyphenated-non-v4-UUID false-positive guards); verified green on real
CI (`rust.yml`'s `test (ubuntu)` job and the full `ci.yml` suite).
**E6's missing counter is fixed (2026-07-02).** Wired
`prefix_drift_detected_total` (real detection logic had existed since
E6 landed; only the counter emission was missing). Deliberately
provider-only, not the spec's literal `{provider, model}` — `model`
is client-controlled input and this crate's own stated cardinality
discipline (`observability::mod` docs: "every label is bounded by
infrastructure config, NOT by request input") already rules out
raw-input labels elsewhere (`service_tier`/`response_status` both
bucket through a validated vocabulary instead); provider-only also
mirrors the existing `proxy_cache_hit_rate_per_session` precedent.
Verified green on real CI (`rust.yml`'s `test (ubuntu)` job and the
full `ci.yml` suite); all 236 `cassandra-proxy --lib` tests plus the
metrics integration suites pass.

**Phase F spot-check result — cannot be trusted as done, real
security-relevant gaps:** F1 (auth-mode classification) is solid. F2's
two most important gates (`cache_control`/`prompt_cache_key` PAYG-only
injection) genuinely work, confirmed by direct code read
(`live_zone_anthropic.rs`, `proxy.rs::maybe_inject_openai_prompt_cache_key`).
*Correction to the sub-agent's initial finding:* it flagged
`auth_mode_policy_enforcement` as defaulting to `Disabled` in
`config.rs:619` — checked directly, that's the **test-only** config
constructor (explicitly commented as such); the real production default
via clap's `default_value_t` at `config.rs:304` is `Enabled`. A stale
comment in `proxy.rs:423` ("Disabled (default until c6/6)") was written
mid-rollout and never updated — that's what misled the first pass. What
*does* hold up: F2's "no lossy compressors for OAuth/Subscription"
requirement (`CompressionPolicy.max_lossy_ratio`, `live_zone_only`) is
genuinely plumbed-but-unconsumed — no compressor reads either field.
**F3's raw-token-storage gap is fixed (2026-07-02).** The spec explicitly
required (see [08-phase-F-auth-mode.md:175](REALIGNMENT/08-phase-F-auth-mode.md))
replacing raw OAuth bearer storage with a one-way hash;
`cassandra/subscription/tracker.py` previously did `self._current_token
= raw`. Now `notify_active()` stores only `_hash_token_id(raw)` (SHA-256
+ last-4 chars) and `_maybe_poll()` always resolves a fresh token via
`read_cached_oauth_token()` rather than reusing the cached copy — no raw
bearer sits in process memory anymore. Verified: all 8
`test_subscription_tracker.py` tests pass, plus a broader
`-k subscription` sweep (105 tests, 1 unrelated pre-existing Windows-only
failure). **F3's other gap — TOIN per-tenant keying — is also fixed
(2026-07-02).** TOIN's `(auth_mode, model_family, structure_hash)`
aggregation key machinery was already real (Phase B PR-B5), but every
call site (`ContentRouter._record_to_toin`, `SmartCrusher`'s own)
called `record_compression()` without passing either value, so every
observation silently landed under `"unknown"`. Threaded both through
the same kwargs-capture mechanism F2.2 already established for
`compression_policy`: `pipeline.py` computes `model_family` once (the
only place `model` is a named parameter rather than a kwarg, via the
existing `cassandra.proxy.output_savings.model_family()` mapper) and
injects it into kwargs; `content_router.py`/`smart_crusher.py` capture
`auth_mode`/`model_family` from kwargs in `apply()` and pass them into
`record_compression()`; `anthropic.py`/`openai.py`'s 6
`compression_policy=compression_policy` call sites now also pass
`auth_mode=` (the raw `AuthMode.value` string already classified at
request entry). Also propagates the context onto the lazily-cached
SmartCrusher instance ContentRouter delegates to directly via
`crush()` (a separate dispatch path that bypasses SmartCrusher's own
`apply()`). Deliberately left F2.2's `toin_read_only` write-gate
untouched — a different, already-documented gap on the same delegated
path, out of this fix's scope. 9 new tests in
`tests/test_toin_auth_mode_keying.py`; verified green on real CI
(`ci.yml`'s `lint` + `test` jobs — the first run's `prefetch-model` job
hit a transient HuggingFace 429 rate-limit unrelated to this change,
confirmed via `gh run rerun --failed` succeeding). **F4 is fixed (2026-07-02).** Added an `AuthMode`
parameter to `headers.rs::build_forward_request_headers`, gated the
whole `X-Forwarded-*`/`X-Request-Id` injection block on `auth_mode !=
Subscription`, and threaded the already-classified `auth_mode` through
all three real call sites (`proxy.rs::forward_http` had it in scope
already; `websocket.rs::ws_handler` bypassed classification entirely
and needed it added; `vertex/raw_predict.rs` hard-codes `OAuth`,
matching an existing PR-E3 rationale that Vertex uses GCP ADC tokens,
not Claude subscription auth). Added the three spec-named integration
tests (`payg_adds_xfwd`, `oauth_adds_xfwd`, `subscription_no_xfwd`) plus
2 new unit tests; verified green on real CI (`rust.yml`'s `test
(ubuntu)` job: `cargo fmt --check`, `cargo clippy`, `cargo test` all
passed), not just locally. **The related accept-encoding gap is fixed
(2026-07-02).** Traced ownership to PR-F2's own file list
(`08-phase-F-auth-mode.md:126`: "`proxy.rs` — accept-encoding strip
becomes conditional on `auth_mode != Subscription`"), which the earlier
pass missed. A blind gate would have reintroduced the exact 502 bug
the strip exists to prevent (an edge proxy forwarding "br, zstd" that
httpx can't decode without the `brotli`/`zstandard` extras, neither of
which is a dependency here) — so instead of stripping, Subscription
mode now filters `accept-encoding` down to codings httpx can actually
decode (`sanitize_accept_encoding_for_subscription` in
`cassandra/proxy/auth_mode.py`), applied at the three call sites where
`auth_mode` is already in scope (`anthropic.py:684`,
`openai.py:1714` and `:2965`). Left the CCR-continuation and generic
Copilot-passthrough strip sites untouched — different code paths,
no `auth_mode` in scope, out of this PR's scope. 8 new unit tests;
verified green on real CI (`ci.yml`, commit `46223f0`).

**Phase G spot-check result:** the strongest phase audited — heavy,
real edge-case test coverage (NaN/infinity/aborted-stream handling in
metrics, multi-worker lock contention in RTK polling). G1 and G2 are
fully genuine. G3 is mostly genuine with one confirmed gap:
`wrap_rtk_tokens_saved_per_session` doesn't exist in code on either
side, despite `docs/rtk-architecture.md` claiming Rust owns it and a
Rust comment claiming Python owns it — both sides think the other
built it. **Confirms Phase I's PR-I9 (cache-hit-rate Prometheus alarm)
is genuinely unblocked**: `proxy_cache_hit_rate_per_session` exists,
is registered, and is wired into all three provider SSE paths with
real edge-case tests.

All of A–G have now been read against their specs (not just
marker-grep). Summary: A and D are genuinely solid. G is nearly
solid (1 minor gap). B, C, E have real-but-survivable gaps. **Phase F
is now fully closed as of 2026-07-02** — every gap found in the
initial audit (F3's raw OAuth token storage AND its TOIN
per-tenant-keying gap, F4's unconditional X-Forwarded-*/X-Request-Id,
and PR-F2's accept-encoding strip) is fixed and CI-verified. F can be
considered fully trustworthy now.

**Phase I scope, read directly from
[11-phase-I-test-infra.md](REALIGNMENT/11-phase-I-test-infra.md)** (10
PRs, meant to land continuously alongside other phases — none have
landed; zero `PR-I*` markers anywhere in code):

| PR | What | Risk | Blocked by | Ready now? |
|---|---|---|---|---|
| I1 | SHA-256 byte-faithful round-trip on real payload — "the single most important regression test for cache safety" | Low | PR-A1 | **Done** (2026-07-02) |
| I2 | SSE corner-case fixtures + 10K-case fuzz/proptest | Low | PR-C1 | **Done** (2026-07-02) |
| I3 | Property tests for compression invariants (determinism, idempotence, token-non-increasing, position/frozen-prefix preservation) | Low | PR-B4 | **Done** (2026-07-02) |
| I4 | Real-traffic shadow test, Python vs Rust, 10K requests, gates Phase H | Medium | PR-A1...PR-G3 (all of A–G) | No — needs G verified first |
| I5 | Promote 3 stubbed parity comparators (`ccr`, `log_compressor`, `cache_aligner`) to real | Medium | PR-B3, PR-B7, PR-A2 | Partial — B3's CodeCompressor gap may not block this (different code path) |
| I6 | Make `make test-parity` a per-PR CI gate (currently nightly, `continue-on-error`) | Low | PR-I5 | No |
| I7 | Cache hot zone non-mutation tests (system/tools/frozen messages byte-equal under compression) | Low | PR-B2 | **Done** (2026-07-02) |
| I8 | Tool-definition byte-stability golden-file snapshots | Low | PR-B7 | **Done** (2026-07-02) |
| I9 | Cache-hit-rate Prometheus alarm | Low | PR-G3 | **Done** (2026-07-02) |
| I10 | Replace fake RTK shim in wrap E2E with real RTK | Low | none | **Done** (2026-07-02) |

**PR-I10 landed (2026-07-02)**, verified green in real CI (`docker-wrap-e2e`
job, not just locally): the spec offered two options — full real-RTK
download in CI, or keep the shim but assert it was genuinely invoked
(not just present on PATH). Took the lower-risk option. Traced which wrap
flows actually shell out to `rtk` first: `wrap openhands --prepare-only`
only does a `shutil.which` lookup (never executes it), while `wrap claude`
genuinely runs `rtk init --global --auto-patch` via `register_claude_hooks`
— that's where the assertion landed. First Phase I PR to actually exist
in code.

**PR-I1 landed (2026-07-02)**, verified green in real CI (`rust.yml`'s
`test (ubuntu)` job, both the full `cargo test --workspace` step and a
new named `cargo test -- byte-faithful gate (PR-I1)` step for fast-fail
visibility). Much of the underlying coverage already existed scattered
across phases (PR-A1's Anthropic passthrough test, Phase C's chat/
responses dispatcher byte-equality assertions, Python's existing
`test_passthrough_no_mutation_byte_equal_sha256`) — what was missing was
the single canonical entry point the spec asks for, plus OpenAI-shaped
recorded fixtures to match the existing Anthropic one. Added
`crates/cassandra-proxy/tests/integration_byte_faithful.rs` (the 4
spec-named tests), two new fixtures (`openai_chat_completions_real.json`,
`openai_responses_real.json` — tool_calls, image content blocks, V4A
patch, local_shell_call, reasoning, compaction), and a `make
test-byte-faithful` target.

**PR-I7 landed (2026-07-02)**, verified green in real CI. Added
`crates/cassandra-proxy/tests/integration_cache_hot_zone.rs` with all 9
spec-named tests. 5 are net new (system/tools/frozen-messages/thinking-
signature/redacted-thinking), built against a shared payload helper
that includes a large compressible `tool_result` in every request
specifically so each test proves the live zone *did* shrink, not just
that nothing ran (a hot-zone assertion that passes vacuously would be a
false sense of safety). The other 4 (reasoning/compaction/v4a/
local_shell — OpenAI Responses opaque items) consolidate existing
`integration_responses.rs` coverage into this canonical file. Added a
`make test-cache-hot-zone` target and a named CI step in `rust.yml`,
same fast-fail-visibility pattern as PR-I1.

**PR-I8 landed (2026-07-02)**, verified green in real CI. Spec named a
Rust file, but tool-definition construction is still Python-owned
(`create_ccr_tool_definition` in `cassandra/ccr/tool_injection.py`,
`ANTHROPIC_CUSTOM_TOOLS` in `cassandra/proxy/memory_tool_adapter.py`)
until Phase H ports it — built `tests/test_tool_def_byte_stability.py`
against the real code instead, documented why in its module docstring.
`cassandra_retrieve` already had solid coverage in
`test_ccr_tool_always_on.py::test_tool_definition_byte_stable`; added
thin golden-file re-verifications for it plus two net-new tests for
`memory_save`/`memory_search`, which had zero prior byte-stability
coverage. Golden files under `tests/golden/tool_defs/` generated
programmatically from real output, not hand-transcribed.

**PR-I3 landed (2026-07-02)**, verified green in real CI. Added
`crates/cassandra-core/tests/proptest_compression.rs` (all 5 spec-named
invariants — determinism, idempotence, token-non-increasing, position
preservation, frozen-prefix integrity — against a generator producing
valid Anthropic request bodies weighted toward occasionally clearing
the live-zone compression threshold, so the properties get exercised
against genuine compression, not just no-ops) and
`crates/cassandra-core/tests/proptest_ccr.rs` (the CCR store round-trip
property, scoped correctly: CCR stores original content by hash for
later retrieval rather than compressing wire bytes itself, so the real
property is `get(put(content).hash) == Some(content)`, not a literal
`decompress(compress(x))`). All 9 property tests pass across 1000
generated cases each — a genuine, positive signal about the actual
correctness of the live-zone dispatcher, not just documentation of
intent.

**PR-I2 landed (2026-07-02)**, verified green in real CI (`rust.yml`'s
`test (ubuntu)` job, new `cargo test -- SSE fixtures + fuzz gate (PR-I2)`
step). Two halves: `crates/cassandra-proxy/tests/proptest_sse.rs` (the
10K-case no-panic property test the spec asks for, run against both a
single-shot push and an arbitrarily-chunked push to directly exercise
the framer's split-UTF-8-across-TCP-chunks contract) and
`crates/cassandra-proxy/tests/integration_sse_fixtures.rs` consuming 10
hand-written fixtures under `tests/fixtures/sse/` — each pinning one
previously-buggy wire-format corner case named in the sse module docs
(P1-8/9/14/15/17, P4-48): out-of-order block/item completion keyed by
index or id, thinking+signature accumulation, mid-stream ping/error
handling, OpenAI's id-only-on-first-chunk tool_call accumulation, a
TCP-drop-before-terminal-event stream (documents that `status` stays
`Open` — the proxy layer must detect the dropped connection itself,
the state machine can't), and a pre-stream 429 returned as
`application/json` (documents that callers must branch on Content-Type
before handing bytes to the SSE framer, not rely on it to sniff
non-SSE input). Added `make test-sse-fixtures`.

**PR-I9 landed (2026-07-02)**, verified green in real CI — and not just
a local YAML parse: added a new `promtool-check` job to `ci.yml`
(gated on `docs/operations/prometheus_rules.yaml` changing) that
downloads the latest `promtool` release fresh each run and actually
validates the rule; it reported `SUCCESS: 1 rules found` on this push.
Added `docs/operations/prometheus_rules.yaml` (a `histogram_quantile`
alert comparing today's rolling 15m p50 against the same window 24h
ago via `offset 1d`, per `provider`) and `docs/operations/runbook.md`
(triage steps ranked by likelihood, tying back to the specific
REALIGNMENT phases most likely to cause a drift).

Remaining unblocked, low-risk Phase I work: **I2** (SSE fuzz fixtures).
I4 (the shadow test that actually gates H1) is likely realistic to
attempt now that Phase F's fingerprint-surface gaps (raw OAuth token
storage, unconditional X-Forwarded-*) are fixed — it was specifically
those gaps that made an OAuth/Subscription-safety shadow test premature
before. I5/I6 remain blocked on Phase B3's CodeCompressor gap.

[12-decisions-needed.md](REALIGNMENT/12-decisions-needed.md) lists 15
decisions the plan originally called blocking for Phase A (ICM deletion
scope, container strategy, the `CASSANDRA_PROXY_BACKEND` cutover switch,
etc.). Given Phase A's PR markers are all present in code, these were
presumably resolved along the way — not independently re-verified here,
worth a quick pass to confirm the doc itself got updated to match.

---

## 7. Product roadmap — making this novel, not just a fork

Ideas discussed, nothing built yet:

- [ ] **PDF/DOCX/PPTX → Markdown extractor** — new `ContentType.DOCUMENT`
      branch in `ContentRouter`, same pattern as the existing
      `html_extractor.py`. Currently a genuine gap: zero PDF/DOCX handling
      exists anywhere in `cassandra/transforms/`.
- [ ] **PII/secret redaction transform** — scrub API keys/tokens/emails
      from tool outputs before they leave the host.
- [ ] **Cross-request document cache** — hash-key CCR entries so a
      re-uploaded doc/PDF compresses once, not per-session.
- [ ] **Per-tool learned compression budgets** — extend TOIN's
      observation-only tracking into an actual per-tool-name budget
      recommendation.
- [ ] **Per-document token-savings receipts** in the dashboard — make
      savings tangible per-artifact, not just aggregate stats.

---

## Suggested sequencing

1. **Section 1 + 3** (infra + governance docs) — done.
2. **Section 6** (REALIGNMENT) — mostly implemented already (A–G). Next:
   spot-check that the PR markers reflect genuinely complete work, then
   tackle Phase H (Python proxy retirement, ~19K LOC) and formalize
   Phase I (test-infra tagging).
3. **Section 7** (product features) — what makes this yours rather than a
   fixed-up fork. Safe to build now that A–G's architecture looks settled.
4. **Section 4** (release readiness) — a 5-minute check you should do
   yourself before the first real publish.
