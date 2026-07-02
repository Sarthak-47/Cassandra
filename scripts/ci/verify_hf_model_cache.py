#!/usr/bin/env python3
"""Verify the HuggingFace model cache is warm before running tests offline.

The `test` job restores `~/.cache/huggingface` from the same cache key the
`prefetch-model` job populates (see .github/workflows/ci.yml), then runs
pytest with HF_HUB_OFFLINE=1 / TRANSFORMERS_OFFLINE=1. If the cache
restore misses or is incomplete, tests that load
sentence-transformers/all-MiniLM-L6-v2 fail deep in the suite with a
confusing offline-mode error. This script fails fast, at the top of the
job, with a diagnostic that actually says what's missing.
"""

from __future__ import annotations

import sys

MODEL_ID = "sentence-transformers/all-MiniLM-L6-v2"


def main() -> int:
    try:
        from huggingface_hub import snapshot_download
    except ImportError:
        # Not every test shard installs the `ml`/`relevance` extra (the base
        # `[dev]` install doesn't pull huggingface_hub) -- if it's not
        # installed, nothing in this job can reach the model anyway, so
        # there's nothing to verify. That's different from "installed but
        # the cache is cold," which IS an error (handled below).
        print(
            "huggingface_hub not installed — this job doesn't need HF model "
            "access, skipping cache verification."
        )
        return 0

    try:
        path = snapshot_download(MODEL_ID, local_files_only=True)
    except Exception as exc:  # huggingface_hub raises several distinct error
        # types (LocalEntryNotFoundError, etc.) depending on what's missing;
        # any of them means the offline cache isn't usable.
        print(
            f"HuggingFace model cache is missing or incomplete for {MODEL_ID!r}: "
            f"{type(exc).__name__}: {exc}\n"
            "This means the `actions/cache` restore for "
            "~/.cache/huggingface (key: <os>-models-allMiniLM-v2) missed, "
            "or the prefetch-model job that populates it hasn't run/succeeded "
            "for this workflow run.",
            file=sys.stderr,
        )
        return 1

    print(f"HuggingFace model cache OK: {MODEL_ID} -> {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
