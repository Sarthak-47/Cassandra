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
code.** Phases A–G are each fully represented (every `PR-A*`...`PR-G*` ID
shows up somewhere in source); the gap is almost entirely Phase H (Python
proxy retirement — only `PR-H2` found, `H1/H3/H4` missing) and Phase I
(test-infra tagging — zero `PR-I*` markers found in `tests/`, though the
~7,700-test suite that exists suggests real test infra just isn't tagged
with these IDs, or Phase I was never formally kicked off as its own effort).

Full detail in [REALIGNMENT/](REALIGNMENT/INDEX.md).

| Phase | What | Status |
|---|---|---|
| A — Lockdown | Stop the cache-busting bugs (passthrough on `/v1/messages`) | **Verified** genuinely done (all 8, one spec-stale note — see below) |
| B — Live-zone engine | Delete ~10K LOC (ICM/scoring/relevance), rebuild compression | **Verified** mostly done, 2 real gaps (relevance/ not deleted, CodeCompressor unwired — see below) |
| C — Rust proxy paths | Port remaining handlers, byte-level SSE parser | **Verified** well-built but not actually deployed (see critical finding above) + 1 dropped feature |
| D — Bedrock/Vertex native | Replace the currently-fake LiteLLM conversion | **Verified** genuinely native (SigV4/EventStream/ADC, real, not a LiteLLM shim) |
| E — Cache stabilization | Deterministic tool/schema ordering | Implemented (6/6 markers) — not yet spot-checked |
| F — Auth-mode policy | PAYG/OAuth/subscription-aware compression | Implemented (4/4 markers) — not yet spot-checked |
| G — RTK + observability | Broader wrap-CLI support, metrics | Implemented (3/3 markers) — not yet spot-checked |
| H — Python retirement | Delete the Python proxy once Rust hits parity | Mostly not started (1/4 — only PR-H2) |
| I — Test infra | SHA-256 round-trip tests, parity gates | Untagged / unclear (0/10 markers, but tests/ has ~7,700 tests) |

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
real implementations with genuine tests. Two real, undocumented gaps:
(1) **PR-B1** — `crates/cassandra-core/src/relevance/` (~1,300 LOC of
BM25/embedding/hybrid scorers, still `pub mod relevance` in `lib.rs`,
actively imported by SmartCrusher/TextCrusher) was never deleted despite
the spec's "~10K LOC delete" target, and the `fastembed` ML dependency
it needs is still live. (2) **PR-B3** — `CodeCompressor` was never wired
into the live-zone dispatcher; `ContentType::SourceCode` is hard-coded
to `NoOp` with an explicit `TODO(PR-B4 / Rust code-compressor port)`
comment, and the Rust test suite was rewritten to assert the no-op
instead of the spec's named `..._routes_to_code_compressor` test —
**meaning source-code tool_results get zero compression today.** Neither
gap looks like negligence (both are consistent, deliberately-coded
no-op branches with matching tests) — reads like an unreflected scope
cut, not a stub.

**Phase C spot-check result:** C1–C3 are solid (byte-level SSE parser,
`/v1/chat/completions`, and an unusually thorough `/v1/responses` item-type
handler in Rust). Two real gaps: (1) **PR-C4** dropped the
Conversations-API-awareness compression-skip feature entirely —
`crates/cassandra-proxy/src/conversations.rs` (detecting `conversation:
{"id": ...}` in `/v1/responses` bodies and skipping live-zone
compression) doesn't exist anywhere, no trace in code or tests; what
exists instead (`handlers/conversations.rs`) is an unrelated
`/v1/conversations*` CRUD passthrough. (2) **PR-C5**'s headline claim —
"after this PR lands, no Python code is on the `/v1/responses` request
path" — is the critical finding above: falsified by the codebase's own
comments (`crates/cassandra-py/src/lib.rs:1560-1569`), since the Rust
proxy binary isn't what `cassandra proxy` deploys.

Next real step: same spot-check treatment for E–G (only A, B, C, D have
been read against spec so far) to know the true completion state before
deciding anything about H.

**Phase I scope, read directly from
[11-phase-I-test-infra.md](REALIGNMENT/11-phase-I-test-infra.md)** (10
PRs, meant to land continuously alongside other phases — none have
landed; zero `PR-I*` markers anywhere in code):

| PR | What | Risk | Blocked by | Ready now? |
|---|---|---|---|---|
| I1 | SHA-256 byte-faithful round-trip on real payload — "the single most important regression test for cache safety" | Low | PR-A1 | **Yes** |
| I2 | SSE corner-case fixtures + 10K-case fuzz/proptest | Low | PR-C1 | **Yes** |
| I3 | Property tests for compression invariants (determinism, idempotence, token-non-increasing, position/frozen-prefix preservation) | Low | PR-B4 | **Yes** |
| I4 | Real-traffic shadow test, Python vs Rust, 10K requests, gates Phase H | Medium | PR-A1...PR-G3 (all of A–G) | No — needs G verified first |
| I5 | Promote 3 stubbed parity comparators (`ccr`, `log_compressor`, `cache_aligner`) to real | Medium | PR-B3, PR-B7, PR-A2 | Partial — B3's CodeCompressor gap may not block this (different code path) |
| I6 | Make `make test-parity` a per-PR CI gate (currently nightly, `continue-on-error`) | Low | PR-I5 | No |
| I7 | Cache hot zone non-mutation tests (system/tools/frozen messages byte-equal under compression) | Low | PR-B2 | **Yes** |
| I8 | Tool-definition byte-stability golden-file snapshots | Low | PR-B7 | **Yes** |
| I9 | Cache-hit-rate Prometheus alarm | Low | PR-G3 | Unknown — G not yet verified |
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

Four PRs (I1, I2, I3, I7, I8) remain low-risk, well-specified, and
unblocked right now — genuinely good next work, in contrast to H1 which
cannot start yet. I4 (the shadow test that actually gates H1) needs E/F/G
verified and possibly built out first.

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
