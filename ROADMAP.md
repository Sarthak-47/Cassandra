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

- [ ] **No root `Dockerfile`.** `docker-native-e2e` fails with "Dockerfile
      not found." This is the main published image
      (`ghcr.io/Sarthak-47/cassandra`) that the Docker-native install docs
      point users at — it has never actually been built.
- [ ] **No `docker-bake.hcl`.** The `Docker` release workflow expects bake
      targets (`runtime`, `runtime-nonroot`, `runtime-code`,
      `runtime-code-nonroot`, `runtime-slim`, `runtime-slim-nonroot`,
      `runtime-code-slim`). Bigger lift than the root Dockerfile — needs
      real design decisions about what goes in "code" vs "slim" variants.
- [ ] **No `.release-please-config.json` / `.release-please-manifest.json`.**
      The `Release Please` workflow (auto version-bump PRs from
      conventional commits) fails: "Missing required manifest config."
- [ ] **`main` has no branch protection.** Anyone can force-push directly.
      At minimum, require the `CI` check to pass before merge once this is
      a project other people touch.

### Done (2026-07-02)
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
  Nothing to do here until litellm ships 3.14 support upstream.
- `[ml]` extras (torch/transformers/sentence-transformers) were never
  pip-audited — the full install was too heavy to complete in one session.
  Worth a follow-up audit pass.
- `mkdocs.yml`'s nav intentionally excludes `wiki/plans/*` (internal
  planning docs) — those pages exist and build, just aren't in the menu.

---

## 3. Governance docs that don't exist

- [ ] `CONTRIBUTING.md` — `.github/copilot-instructions.md` explicitly
      tells PR reviewers to treat it as "required policy," but it doesn't
      exist yet, so that instruction currently points at nothing.
- [ ] `CODE_OF_CONDUCT.md`
- [ ] `SECURITY.md` — matters more than usual here since this project is a
      proxy that handles API keys and provider credentials.

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

## 6. The compression-engine rewrite (REALIGNMENT) — 0% started

Full detail in [REALIGNMENT/](REALIGNMENT/INDEX.md). This is the reason a
lot of what's in section 1 existed to fix in the first place — the plan
was written against a codebase that had already drifted from it.
Confirmed via `git log`: **zero `realign-*` commits exist anywhere.**

| Phase | What | Status |
|---|---|---|
| A — Lockdown | Stop the cache-busting bugs (passthrough on `/v1/messages`) | Not started |
| B — Live-zone engine | Delete ~10K LOC (ICM/scoring/relevance), rebuild compression | Not started |
| C — Rust proxy paths | Port remaining handlers, byte-level SSE parser | Not started |
| D — Bedrock/Vertex native | Replace the currently-fake LiteLLM conversion | Not started |
| E — Cache stabilization | Deterministic tool/schema ordering | Not started |
| F — Auth-mode policy | PAYG/OAuth/subscription-aware compression | Not started |
| G — RTK + observability | Broader wrap-CLI support, metrics | Not started |
| H — Python retirement | Delete the Python proxy once Rust hits parity | Not started |
| I — Test infra | SHA-256 round-trip tests, parity gates | Not started |

Blocking Phase A: **15 unresolved decisions** in
[12-decisions-needed.md](REALIGNMENT/12-decisions-needed.md) — ICM
deletion scope, container strategy, the `CASSANDRA_PROXY_BACKEND` cutover
switch, etc. Nobody's signed off on any of them yet.

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

1. **Section 1 + 3** (infra + governance docs) — small, mechanical, hours
   not days. Low risk, do these whenever.
2. **Section 6** (REALIGNMENT) — the real project. Weeks of work, and it's
   what determines whether the compression engine is actually correct.
   Start by resolving the 15 open decisions, then Phase A.
3. **Section 7** (product features) — what makes this yours rather than a
   fixed-up fork. Can run in parallel with REALIGNMENT once Phase A lands
   (don't build new compressors on top of the architecture that's about to
   be deleted in Phase B).
4. **Section 4** (release readiness) — a 5-minute check you should do
   yourself before the first real publish.
