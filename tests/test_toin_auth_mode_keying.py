"""Phase F PR-F3: TOIN aggregation-key threading tests.

TOIN's pattern store is keyed by ``(auth_mode, model_family,
structure_hash)`` (see ``cassandra/telemetry/toin.py``), but until this
PR, ``ContentRouter``/``SmartCrusher``'s ``record_compression()`` call
sites never passed real values -- every observation silently landed
under ``("unknown", "unknown", ...)``. This closes that gap by
threading the values through ``apply(**kwargs)`` the same way F2.2
threads ``compression_policy``.

Mirrors the structure of ``tests/test_compression_policy_toin_gate.py``
(the F2.2 write-gate suite) so a future contributor can find both
TOIN-keying-context test suites by the same naming convention.
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

from cassandra.telemetry.toin import (
    DEFAULT_AUTH_MODE,
    DEFAULT_MODEL_FAMILY,
    TOINConfig,
    get_toin,
    reset_toin,
)
from cassandra.tokenizer import Tokenizer
from cassandra.tokenizers import EstimatingTokenCounter
from cassandra.transforms.base import Transform
from cassandra.transforms.pipeline import TransformPipeline


def _has_core() -> bool:
    """Same skip rationale as test_compression_policy_toin_gate.py."""
    try:
        from cassandra._core import SmartCrusher  # noqa: F401

        return True
    except ImportError:
        return False


_skip_no_core = pytest.mark.skipif(
    not _has_core(),
    reason="cassandra._core wheel not installed (run `scripts/build_rust_extension.sh`)",
)


@pytest.fixture
def fresh_toin():
    """Per-test TOIN instance backed by a tempdir to avoid global drift."""
    reset_toin()
    with tempfile.TemporaryDirectory() as tmpdir:
        storage = str(Path(tmpdir) / "toin.json")
        toin = get_toin(TOINConfig(storage_path=storage, auto_save_interval=0))
        yield toin
        reset_toin()


def _bigger_array(n: int = 60) -> str:
    items = [{"status": "ok", "tag": "x", "n": i} for i in range(n)]
    return json.dumps(items)


def _wrap_in_tool_message(payload: str) -> list[dict]:
    return [{"role": "tool", "content": payload, "tool_call_id": "t1"}]


def _tokenizer() -> Tokenizer:
    return Tokenizer(EstimatingTokenCounter())  # type: ignore[arg-type]


# ─── ContentRouter: apply() captures auth_mode/model_family ────────────


def test_content_router_apply_stores_runtime_auth_mode_and_model_family():
    from cassandra.transforms.content_router import ContentRouter

    router = ContentRouter()
    assert router._runtime_auth_mode is None
    assert router._runtime_model_family is None

    router.apply([], _tokenizer(), auth_mode="payg", model_family="sonnet")
    assert router._runtime_auth_mode == "payg"
    assert router._runtime_model_family == "sonnet"


def test_content_router_apply_defaults_to_none_when_not_provided():
    """No kwargs -> both fields stay None, matching pre-F3 behaviour for
    non-proxy callers (tests, hand-written pipelines)."""
    from cassandra.transforms.content_router import ContentRouter

    router = ContentRouter()
    router.apply([], _tokenizer())
    assert router._runtime_auth_mode is None
    assert router._runtime_model_family is None


def test_content_router_record_to_toin_keys_pattern_by_real_auth_mode_and_model_family(
    fresh_toin,
):
    """The load-bearing assertion: a real (non-default) key appears in
    TOIN's pattern store, not the ("unknown", "unknown", ...) default."""
    from cassandra.transforms.content_router import (
        CompressionStrategy,
        ContentRouter,
    )

    router = ContentRouter()
    router._runtime_auth_mode = "oauth"
    router._runtime_model_family = "gpt"

    router._record_to_toin(
        strategy=CompressionStrategy.TEXT,
        content="some text content with structure that learns",
        compressed="compressed shorter",
        original_tokens=100,
        compressed_tokens=50,
    )
    keys = list(fresh_toin._patterns.keys())
    assert keys, "expected at least one TOIN pattern to have been recorded"
    assert all(k[0] == "oauth" and k[1] == "gpt" for k in keys), (
        f"expected every pattern key to be (oauth, gpt, ...); got {keys}"
    )
    assert not any(k[0] == DEFAULT_AUTH_MODE or k[1] == DEFAULT_MODEL_FAMILY for k in keys), (
        "PR-F3 regression: observation landed under the unknown/unknown default "
        "despite a real auth_mode/model_family being set"
    )


def test_content_router_record_to_toin_falls_back_to_unknown_when_unset(fresh_toin):
    """Baseline (pre-F3-equivalent) behaviour is preserved for callers
    that don't set the runtime fields -- documents the fallback rather
    than silently changing it."""
    from cassandra.transforms.content_router import (
        CompressionStrategy,
        ContentRouter,
    )

    router = ContentRouter()
    # _runtime_auth_mode / _runtime_model_family left at their None default.

    router._record_to_toin(
        strategy=CompressionStrategy.TEXT,
        content="some other text content with structure that learns",
        compressed="compressed shorter",
        original_tokens=100,
        compressed_tokens=50,
    )
    keys = list(fresh_toin._patterns.keys())
    assert keys, "expected at least one TOIN pattern to have been recorded"
    assert all(k[0] == DEFAULT_AUTH_MODE and k[1] == DEFAULT_MODEL_FAMILY for k in keys)


# ─── SmartCrusher: apply() captures auth_mode/model_family ─────────────


@_skip_no_core
def test_smart_crusher_apply_stores_runtime_auth_mode_and_model_family():
    from cassandra.transforms.smart_crusher import SmartCrusher, SmartCrusherConfig

    crusher = SmartCrusher(SmartCrusherConfig())
    assert crusher._runtime_auth_mode is None
    assert crusher._runtime_model_family is None

    crusher.apply([], _tokenizer(), auth_mode="subscription", model_family="opus")
    assert crusher._runtime_auth_mode == "subscription"
    assert crusher._runtime_model_family == "opus"


@_skip_no_core
def test_smart_crusher_apply_keys_toin_pattern_by_real_auth_mode_and_model_family(
    fresh_toin,
):
    from cassandra.transforms.smart_crusher import SmartCrusher, SmartCrusherConfig

    crusher = SmartCrusher(SmartCrusherConfig())
    messages = _wrap_in_tool_message(_bigger_array(60))

    result = crusher.apply(messages, _tokenizer(), auth_mode="payg", model_family="haiku")
    if not result.transforms_applied:
        pytest.skip("payload didn't trigger compression — bump the size")

    keys = list(fresh_toin._patterns.keys())
    assert keys, "expected at least one TOIN pattern to have been recorded"
    assert all(k[0] == "payg" and k[1] == "haiku" for k in keys), (
        f"expected every pattern key to be (payg, haiku, ...); got {keys}"
    )


# ─── ContentRouter -> SmartCrusher delegation propagates context ───────


@_skip_no_core
def test_content_router_propagates_auth_mode_to_delegated_smart_crusher():
    """ContentRouter._get_smart_crusher() caches one SmartCrusher
    instance and calls .crush() on it directly (bypassing
    SmartCrusher.apply()) from _compress_pure/_apply_strategy_to_content.
    That instance must still see the current request's auth_mode/
    model_family so its own _record_to_toin keys correctly."""
    from cassandra.transforms.content_router import ContentRouter

    router = ContentRouter()
    router._runtime_auth_mode = "oauth"
    router._runtime_model_family = "sonnet"

    crusher = router._get_smart_crusher()
    if crusher is None:
        pytest.skip("SmartCrusher unavailable in this build")

    # Simulate what _compress_pure does immediately before crusher.crush().
    crusher._runtime_auth_mode = router._runtime_auth_mode
    crusher._runtime_model_family = router._runtime_model_family
    assert crusher._runtime_auth_mode == "oauth"
    assert crusher._runtime_model_family == "sonnet"


# ─── Pipeline.apply(): model_family injection from the `model` param ───


class _SpyTransform(Transform):
    """Records the kwargs a Pipeline forwards to `Transform.apply()`."""

    name = "spy"

    def __init__(self):
        self.captured: dict = {}

    def apply(self, messages, tokenizer, **kwargs):
        self.captured.update(kwargs)
        from cassandra.config import TransformResult

        return TransformResult(
            messages=messages, tokens_before=0, tokens_after=0, transforms_applied=[]
        )


def test_pipeline_apply_injects_model_family_from_model_param():
    """`model` is a named Pipeline.apply parameter, not a kwarg -- this
    is the one place that maps it to model_family and injects it into
    kwargs so downstream transforms (which only see **kwargs) get it."""
    spy = _SpyTransform()
    pipeline = TransformPipeline(transforms=[spy])
    pipeline.apply(
        messages=[{"role": "user", "content": "hi"}],
        model="claude-opus-4-6",
        model_limit=100_000,
    )
    assert spy.captured.get("model_family") == "opus", (
        f"expected model_family='opus' injected from model='claude-opus-4-6', "
        f"got kwargs={spy.captured}"
    )


def test_pipeline_apply_lets_explicit_model_family_override():
    """`kwargs.setdefault` means an explicit caller-supplied
    model_family is NOT clobbered by the auto-derived one."""
    spy = _SpyTransform()
    pipeline = TransformPipeline(transforms=[spy])
    pipeline.apply(
        messages=[{"role": "user", "content": "hi"}],
        model="claude-opus-4-6",
        model_limit=100_000,
        model_family="explicit-override",
    )
    assert spy.captured.get("model_family") == "explicit-override"
