"""REALIGNMENT Phase I PR-I8 — tool-definition byte-stability snapshots.

For every tool definition Cassandra auto-injects (``cassandra_retrieve``,
``memory_save``, ``memory_search``), pin the canonical serialized bytes
via a golden file under ``tests/golden/tool_defs/``. Any accidental
change to a definition's field order, wording, or schema shape busts
the client's prompt cache for every active session the moment it ships
— this suite exists to make that a *loud, deliberate* choice (update the
golden file) rather than a silent regression.

Note on file location: REALIGNMENT/11-phase-I-test-infra.md names this
suite as a Rust file
(``crates/cassandra-core/tests/tool_def_byte_stability.rs``) with golden
files under ``crates/cassandra-core/tests/golden/tool_defs/``. That
assumed Rust already owned tool-definition construction; as of this
session's Phase C/H audit it doesn't yet — ``create_ccr_tool_definition``
lives in ``cassandra/ccr/tool_injection.py`` and the memory tool schemas
are the ``ANTHROPIC_CUSTOM_TOOLS`` constant in
``cassandra/proxy/memory_tool_adapter.py``. Building this in Python
against the code that actually constructs these definitions protects
something real; a Rust test file for Python-owned logic would not. This
should move to Rust once Phase H ports tool-definition construction
there.

The `cassandra_retrieve` (CCR) byte-stability invariant already has
dedicated coverage in ``test_ccr_tool_always_on.py::
test_tool_definition_byte_stable`` (tied to the PR-B7 session-stickiness
feature it protects); the two CCR tests here are thin, golden-file-based
re-verifications so this file is a complete, standalone entry point per
the spec, not a replacement for that richer test.
"""

from __future__ import annotations

from pathlib import Path

from cassandra.ccr.tool_injection import create_ccr_tool_definition
from cassandra.proxy.helpers import serialize_tool_definition_canonical
from cassandra.proxy.memory_tool_adapter import ANTHROPIC_CUSTOM_TOOLS

GOLDEN_DIR = Path(__file__).resolve().parent / "golden" / "tool_defs"


def _assert_byte_stable(golden_filename: str, actual: bytes) -> None:
    golden_path = GOLDEN_DIR / golden_filename
    expected = golden_path.read_bytes()
    assert actual == expected, (
        f"{golden_filename}: tool definition bytes changed.\n"
        f"  expected: {expected!r}\n"
        f"  actual:   {actual!r}\n"
        f"If this change is intentional, update {golden_path} and review the "
        f"prompt-cache implications (every active session's cached prefix "
        f"breaks the moment this ships)."
    )


def test_ccr_retrieve_definition_anthropic_byte_stable() -> None:
    actual = serialize_tool_definition_canonical(create_ccr_tool_definition("anthropic"))
    _assert_byte_stable("ccr_retrieve_anthropic.json", actual)


def test_ccr_retrieve_definition_openai_byte_stable() -> None:
    actual = serialize_tool_definition_canonical(create_ccr_tool_definition("openai"))
    _assert_byte_stable("ccr_retrieve_openai.json", actual)


def test_memory_save_definition_byte_stable() -> None:
    memory_save = next(t for t in ANTHROPIC_CUSTOM_TOOLS if t["name"] == "memory_save")
    actual = serialize_tool_definition_canonical(memory_save)
    _assert_byte_stable("memory_save_anthropic.json", actual)


def test_memory_search_definition_byte_stable() -> None:
    memory_search = next(t for t in ANTHROPIC_CUSTOM_TOOLS if t["name"] == "memory_search")
    actual = serialize_tool_definition_canonical(memory_search)
    _assert_byte_stable("memory_search_anthropic.json", actual)


def test_renaming_a_field_fails_ci() -> None:
    """Acceptance criterion: renaming a field in a tool definition fails CI.

    Not a real production code path -- directly proves the golden-file
    comparator actually discriminates a changed schema, so this suite
    can't silently pass no matter what `create_ccr_tool_definition`
    returns.
    """
    mutated = create_ccr_tool_definition("anthropic")
    mutated["input_schema"]["properties"]["hash_renamed"] = mutated["input_schema"][
        "properties"
    ].pop("hash")
    mutated_bytes = serialize_tool_definition_canonical(mutated)
    golden = (GOLDEN_DIR / "ccr_retrieve_anthropic.json").read_bytes()
    assert mutated_bytes != golden, "renaming a field must change the canonical bytes"
