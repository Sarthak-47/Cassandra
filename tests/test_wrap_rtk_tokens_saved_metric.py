"""Phase G PR-G3 remediation (C4, part 2) — `wrap_rtk_tokens_saved_per_session`.

`REALIGNMENT/09-phase-G-rtk-observability.md` named this metric but
neither side ever exported it: the Rust proxy's metric-name constants
were removed (see `crates/cassandra-proxy/src/observability/
metric_names.rs`'s comment on this) once it was decided the natural
owner is Python, since PR-G2 already wires `tokens_saved_rtk` into
`SubscriptionTracker` end-to-end. This file pins the Python export
added to `cassandra/proxy/prometheus_metrics.py`.

The spec calls it a histogram "populated at session end"; this proxy
runs as a child process of a single `cassandra wrap <agent>` session
(see `docs/rtk-architecture.md`), so there's no later scrape after
"session end" to observe a bucket into. A gauge exposing the *current*
session's live cumulative RTK savings is what an operator dashboard
can actually read mid-session, so that's what's exported here — same
cardinality/shape-pragmatism precedent as `prefix_drift_detected_total`
and `proxy_conversations_api_request_count_total` on the Rust side.
"""

from __future__ import annotations

import pytest

import cassandra.subscription.tracker as tracker_module
from cassandra.proxy.prometheus_metrics import PrometheusMetrics
from cassandra.subscription.tracker import SubscriptionTracker


def _build_tracker(monkeypatch: pytest.MonkeyPatch) -> SubscriptionTracker:
    """Construct a tracker with persistence disabled, mirroring the
    pattern in test_subscription_tracker_rtk_wired.py::_build_tracker."""
    monkeypatch.setattr(SubscriptionTracker, "_load_persisted_state", lambda self: None)
    return SubscriptionTracker(enabled=True)


@pytest.mark.asyncio
async def test_gauge_zero_when_no_tracker_configured(monkeypatch: pytest.MonkeyPatch) -> None:
    """No subscription tracker singleton configured (e.g. tracking
    disabled) -> the metric is emitted as zero, not omitted, so the
    family always advertises HELP/TYPE from boot."""
    monkeypatch.setattr(tracker_module, "_tracker_instance", None)
    metrics = PrometheusMetrics()
    body = await metrics.export()
    assert "# TYPE wrap_rtk_tokens_saved_per_session gauge" in body
    assert "wrap_rtk_tokens_saved_per_session 0" in body


@pytest.mark.asyncio
async def test_gauge_reflects_tracker_cumulative_rtk_savings(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """Once the tracker has accumulated RTK savings this session, the
    exported gauge reflects the same `tokens_saved["rtk_raw"]` value
    the /stats dashboard reads."""
    tracker = _build_tracker(monkeypatch)
    tracker.update_contribution(tokens_saved_rtk=250)
    tracker.update_contribution(tokens_saved_rtk=130)
    monkeypatch.setattr(tracker_module, "_tracker_instance", tracker)

    metrics = PrometheusMetrics()
    body = await metrics.export()

    assert "wrap_rtk_tokens_saved_per_session 380" in body
    assert tracker.state["contribution"]["tokens_saved"]["rtk_raw"] == 380
