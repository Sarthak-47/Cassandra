# Runbook: `CassandraCacheHitRateDriftBelowBaseline`

REALIGNMENT Phase I PR-I9. Alert rule defined in
[`prometheus_rules.yaml`](./prometheus_rules.yaml).

## What fired

`proxy_cache_hit_rate_per_session`'s rolling 15-minute p50 for a given
`provider` (`anthropic`, `openai_chat`, or `openai_responses`) has been
below 90% of the same 15-minute window 24 hours ago, sustained for more
than 15 minutes.

This metric is the **Phase H canary gate** — see
[`10-phase-H-python-retirement.md`](../../REALIGNMENT/10-phase-H-python-retirement.md).
A real drift here means real money: every point of cache-hit-rate lost
is tokens re-read at full price instead of the cached-read discount.

## Why this fires (in rough order of likelihood)

1. **A code change mutated the cache hot zone.** The most likely and
   most serious cause. Something in `system`, `tools`, the frozen
   `messages` prefix, or an opaque round-trip type (thinking
   signatures, redacted-thinking data, OpenAI Responses reasoning/
   compaction/apply_patch/local_shell items) changed byte-for-byte
   between requests in the same session, busting the provider's prompt
   cache. Check recent deploys against
   `crates/cassandra-proxy/tests/integration_cache_hot_zone.rs` and
   `crates/cassandra-proxy/tests/integration_byte_faithful.rs` — if
   either would now fail, that's your regression; `git bisect` the
   deploy window.
2. **Non-deterministic tool/schema ordering.** Phase E's tool-array and
   JSON-schema-key sort
   (`crates/cassandra-proxy/src/cache_stabilization/tool_def_normalize.rs`)
   must be stable across turns. A regression there reorders what the
   client considers cache-stable content.
3. **`X-Forwarded-*` / header drift on Subscription-mode traffic.**
   Phase F PR-F4 gates this — if `auth_mode` classification broke (see
   `crates/cassandra-core/src/auth_mode.rs`), previously-suppressed
   fingerprint headers could start reaching the upstream again,
   changing what the provider sees as "the same client."
4. **A genuine upstream provider change**, not a Cassandra regression
   — e.g. the provider changed cache TTL, cache-eligibility rules, or
   is having an incident. Check the provider's status page and compare
   against a raw (non-Cassandra) baseline if you have one.
5. **A real, legitimate traffic-mix shift** — e.g. a new integration
   sends much shorter conversations that never build up enough cached
   prefix to benefit. The `offset 1d` comparison already controls for
   day-of-week effects; if this recurs at the same time for multiple
   days running, it may be traffic, not a bug.

## Triage steps

1. **Confirm it's not noise.** Check the Grafana panel for
   `proxy_cache_hit_rate_per_session` (or query it directly) over the
   last 48h for the affected `provider`. A brief dip that self-resolves
   before `for: 15m` elapses won't fire; if it did fire, the drift was
   sustained.
2. **Correlate with deploys.** Cross-reference the alert's start time
   against recent merges to `main`. The REALIGNMENT phases most likely
   to cause this (A, B, E, F — see above) all touch the hot zone or
   header/ordering determinism.
3. **Check `proxy_passthrough_bytes_modified_total`** (see
   [`observability.md`](../observability.md)) — this metric alarms
   independently on any non-zero mutation rate outside the compression
   hot path and is a more direct signal than the hit-rate drift itself.
   If it's also non-zero, you have byte-level proof of a cache-busting
   bug, not just a correlation.
4. **If a regression is confirmed:** revert the offending deploy.
   Prompt-cache damage is not retroactively fixable — the provider's
   cache for already-broken sessions is already cold; reverting stops
   the bleeding for *new* sessions going forward.
5. **If it's a genuine provider-side issue:** file with the provider,
   note it in the incident channel, and consider silencing this alert
   for the affected `provider` label until resolved (don't silence the
   whole rule — other providers may still be healthy).

## False-positive conditions

- Low absolute traffic volume for a `provider` makes the `rate()`
  windows noisy — a handful of unusually short sessions can swing the
  p50 without any regression. Check `proxy_cache_hit_rate_per_session`'s
  `_count` alongside the alert before treating it as urgent.
- A newly-onboarded integration with a different usage pattern (see
  reason 5 above) can look identical to a regression for its first
  24-48h until the `offset 1d` baseline catches up to the new normal.
