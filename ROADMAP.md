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

## 6. The compression-engine rewrite (REALIGNMENT) — ~76% implemented

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
| A — Lockdown | Stop the cache-busting bugs (passthrough on `/v1/messages`) | **Verified** genuinely implemented (8/8, real logic + tests, not stubs) |
| B — Live-zone engine | Delete ~10K LOC (ICM/scoring/relevance), rebuild compression | Implemented (7/7 markers) — not yet spot-checked |
| C — Rust proxy paths | Port remaining handlers, byte-level SSE parser | Implemented (5/5 markers) — not yet spot-checked |
| D — Bedrock/Vertex native | Replace the currently-fake LiteLLM conversion | Implemented (5/5 markers) — not yet spot-checked |
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
Verdict: genuinely done, no stubs/no-ops/TODOs blocking any of them.
Two notes: (1) several implementations live in different files/functions
than the spec names — drift in location, not in substance; (2) PR-A1's
"pure passthrough stub" has already been superseded by live Phase B
compression code gated behind `CompressionMode` — the codebase is ahead
of Phase A's literal spec text, not behind it.

Next real step: same spot-check treatment for B–G (only A has been
read against its spec so far — B–G's "Implemented" status still rests
on marker-grep alone, which only proves *a PR landed*, not that it
fully satisfies its phase's spec). Then audit whether the Python proxy
(`cassandra/proxy/`, ~19K LOC) can actually be retired now that A–G
*claim* completion.

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
