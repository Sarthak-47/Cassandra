"""OpenCode config file helpers for wrap and persistent install."""

from __future__ import annotations

import json
import os
import re
import shutil
from pathlib import Path
from typing import Any

import click

from cassandra import fsutil
from cassandra.install.paths import opencode_config_path
from cassandra.mcp_registry.install import DEFAULT_PROXY_URL

# Cassandra-managed JSON marker comments for idempotent block injection.
_PROVIDER_MARKER_START = "// --- Cassandra proxy provider ---"
_PROVIDER_MARKER_END = "// --- end Cassandra proxy provider ---"
_MCP_MARKER_START = "// --- Cassandra MCP server ---"
_MCP_MARKER_END = "// --- end Cassandra MCP server ---"

# Regex to strip cassandra blocks (including the marker comments).
_PROVIDER_BLOCK_RE = re.compile(
    re.escape(_PROVIDER_MARKER_START) + r".*?" + re.escape(_PROVIDER_MARKER_END),
    re.DOTALL,
)
_MCP_BLOCK_RE = re.compile(
    re.escape(_MCP_MARKER_START) + r".*?" + re.escape(_MCP_MARKER_END),
    re.DOTALL,
)
CASSANDRA_OPENCODE_PLUGIN = "cassandra-opencode"


def _opencode_home_dir() -> Path:
    """Return the OpenCode home/config directory."""
    env_path = os.environ.get("OPENCODE_HOME", "").strip()
    if env_path:
        return Path(env_path).expanduser()
    return Path.home() / ".config" / "opencode"


def opencode_config_paths() -> tuple[Path, Path]:
    """Return ``(config_file, backup_file)`` for OpenCode."""
    config_file = opencode_config_path()
    backup_file = config_file.with_suffix(".json.cassandra-backup")
    return config_file, backup_file


def snapshot_opencode_config_if_unwrapped(config_file: Path, backup_file: Path) -> None:
    """Snapshot ``opencode.json`` to ``backup_file`` before the first injection.

    Guarantees that ``cassandra unwrap opencode`` can restore the user's
    original file byte-for-byte.
    """
    if backup_file.exists():
        return
    if not config_file.exists():
        return
    try:
        content = fsutil.read_text(config_file)
    except OSError:
        return
    if _PROVIDER_MARKER_START in content or _MCP_MARKER_START in content:
        return
    backup_file.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(config_file, backup_file)


def strip_opencode_cassandra_blocks(content: str, *, remove_mcp: bool = True) -> str:
    """Remove all Cassandra-managed blocks from opencode JSON text.

    Preserves user content. Returns the cleaned string.
    """
    content = _PROVIDER_BLOCK_RE.sub("", content)
    if remove_mcp:
        content = _MCP_BLOCK_RE.sub("", content)
    # Collapse multiple blank lines left behind by block removal.
    content = re.sub(r"\n{3,}", "\n\n", content)
    return content.strip()


def _render_provider_block(port: int) -> str:
    """Render a Cassandra provider block as a JSON comment-wrapped snippet."""
    provider = {
        "cassandra": {
            "npm": "@ai-sdk/openai-compatible",
            "name": "Cassandra Proxy",
            "options": {"baseURL": f"http://127.0.0.1:{port}/v1"},
        }
    }
    lines = [
        _PROVIDER_MARKER_START,
        f'"provider": {json.dumps(provider, indent=2)},',
        _PROVIDER_MARKER_END,
    ]
    return "\n".join(lines)


def _render_mcp_block(port: int) -> str:
    """Render a Cassandra MCP block as a JSON comment-wrapped snippet."""
    proxy_url = f"http://127.0.0.1:{port}"
    mcp_entry: dict[str, Any] = {
        "type": "local",
        "command": ["cassandra", "mcp", "serve"],
        "enabled": True,
    }
    if proxy_url != DEFAULT_PROXY_URL:
        mcp_entry["environment"] = {"CASSANDRA_PROXY_URL": proxy_url}
    mcp = {
        "cassandra": mcp_entry,
    }
    lines = [
        _MCP_MARKER_START,
        f'"mcp": {json.dumps(mcp, indent=2)},',
        _MCP_MARKER_END,
    ]
    return "\n".join(lines)


def _parse_json_loose(text: str) -> dict[str, Any]:
    """Parse JSON text, stripping line comments (// ...) when needed.

    Tries standard JSON first to avoid corrupting URLs that contain ``//``.
    Falls back to stripping ``//`` comments when standard parsing fails.
    Two-pass: (1) remove comment-only lines, (2) strip inline trailing
    comments that follow a comma.
    """
    try:
        parsed = json.loads(text)
        return parsed if isinstance(parsed, dict) else {}
    except json.JSONDecodeError:
        pass
    # Pass 1: remove lines that are ONLY a comment.
    cleaned = re.sub(r"^\s*//[^\n]*\n", "", text, flags=re.MULTILINE)
    # Pass 2: remove inline trailing comments (", // comment").
    cleaned = re.sub(r",\s*//[^\n]*", ",", cleaned)
    try:
        parsed = json.loads(cleaned)
        return parsed if isinstance(parsed, dict) else {}
    except json.JSONDecodeError:
        return {}


def _inject_key_into_json(data: dict[str, Any], key: str, value: Any) -> dict[str, Any]:
    """Merge ``value`` into ``data[key]`` idempotently."""
    existing = data.get(key)
    if isinstance(existing, dict) and isinstance(value, dict):
        merged = {**existing, **value}
        data[key] = merged
    else:
        data[key] = value
    return data


def append_cassandra_plugin(config: dict[str, object]) -> bool:
    """Append the optional OpenCode plugin entry if it is not already present."""
    plugin = config.get("plugin")
    if plugin is None:
        config["plugin"] = [CASSANDRA_OPENCODE_PLUGIN]
        return True

    if not isinstance(plugin, list):
        return False

    for entry in plugin:
        if entry == CASSANDRA_OPENCODE_PLUGIN:
            return False
        if isinstance(entry, list) and entry and entry[0] == CASSANDRA_OPENCODE_PLUGIN:
            return False

    plugin.append(CASSANDRA_OPENCODE_PLUGIN)
    return True


def inject_opencode_provider_config(port: int) -> None:
    """Inject a Cassandra model provider into OpenCode's config file.

    Safe to call multiple times — the injected block is fully replaced on
    each call, so re-running with a different ``port`` updates the config.
    Before the first injection, the pre-wrap file is snapshotted to
    ``opencode.json.cassandra-backup`` so ``cassandra unwrap opencode``
    can restore it byte-for-byte.
    """
    config_file, backup_file = opencode_config_paths()
    config_dir = config_file.parent

    try:
        config_dir.mkdir(parents=True, exist_ok=True)
        snapshot_opencode_config_if_unwrapped(config_file, backup_file)

        if config_file.exists():
            content = fsutil.read_text(config_file)
            data = _parse_json_loose(content)
        else:
            content = ""
            data = {}

        # Strip any prior Cassandra-managed blocks before re-injecting.
        if _PROVIDER_MARKER_START in content or _MCP_MARKER_START in content:
            content = strip_opencode_cassandra_blocks(content)
            data = _parse_json_loose(content)

        # Merge provider into the JSON data structure.
        provider = {
            "cassandra": {
                "npm": "@ai-sdk/openai-compatible",
                "name": "Cassandra Proxy",
                "options": {"baseURL": f"http://127.0.0.1:{port}/v1"},
            }
        }
        data = _inject_key_into_json(data, "provider", provider)

        # Write back as formatted JSON (opencode uses standard JSON with comments).
        output = json.dumps(data, indent=2) + "\n"
        config_file.write_text(output, encoding="utf-8")
    except OSError as exc:
        raise click.ClickException(
            f"could not write OpenCode config at {config_file}: {exc}"
        ) from exc
