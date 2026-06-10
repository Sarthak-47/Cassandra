from __future__ import annotations

import base64
import json
from pathlib import Path

from click.testing import CliRunner

from cassandra.capture.network_diff import (
    compare_captures,
    load_capture_file,
    render_markdown_report,
)
from cassandra.cli.main import main


def _write_jsonl(path: Path, records: list[dict[str, object]]) -> None:
    path.write_text("\n".join(json.dumps(record) for record in records) + "\n", encoding="utf-8")


def _body(payload: dict[str, object]) -> str:
    return base64.b64encode(json.dumps(payload).encode("utf-8")).decode("ascii")


def test_network_diff_redacts_and_reports_body_json_deltas(tmp_path: Path) -> None:
    direct_path = tmp_path / "direct.jsonl"
    cassandra_path = tmp_path / "cassandra.jsonl"
    _write_jsonl(
        direct_path,
        [
            {
                "lane": "direct",
                "method": "POST",
                "url": "https://api.anthropic.com/v1/messages?api_key=secret",
                "request_headers": {
                    "authorization": "Bearer secret",
                    "anthropic-version": "2023-06-01",
                    "anthropic-beta": "deferred-tools",
                },
                "request_body_b64": _body(
                    {"model": "claude", "messages": [{"content": "hi"}], "tools": []}
                ),
                "response_status": 200,
            }
        ],
    )
    _write_jsonl(
        cassandra_path,
        [
            {
                "lane": "cassandra",
                "method": "POST",
                "url": "https://api.anthropic.com/v1/messages?api_key=secret",
                "request_headers": {
                    "authorization": "Bearer other",
                    "anthropic-version": "2023-06-01",
                    "x-cassandra-mode": "optimize",
                },
                "request_body_b64": _body(
                    {
                        "model": "claude",
                        "messages": [{"content": "hello"}],
                        "metadata": {},
                        "tools": [{"name": "ctx_execute", "input_schema": {"type": "object"}}],
                    }
                ),
                "response_status": 200,
            }
        ],
    )

    direct = load_capture_file(direct_path, fallback_lane="direct")
    cassandra = load_capture_file(cassandra_path, fallback_lane="cassandra")

    assert direct[0].url == "https://api.anthropic.com/v1/messages?api_key=%3Credacted%3E"
    assert direct[0].request_headers["authorization"] == "<redacted>"

    diff = compare_captures(direct, cassandra)
    assert diff.direct_count == 1
    assert diff.cassandra_count == 1
    paired = diff.paired[0]
    assert paired["headers"]["only_cassandra"] == ["x-cassandra-mode"]
    assert "$.metadata" in paired["json"]["only_cassandra"]
    assert "$.messages[0].content" in paired["json"]["changed"]
    assert paired["anthropic"]["direct"]["tools_count"] == 0
    assert paired["anthropic"]["cassandra"]["tools_count"] == 1

    markdown = render_markdown_report(diff)
    assert "Differential Network Capture Report" in markdown
    assert "POST api.anthropic.com/v1/messages?api_key=%3Credacted%3E" in markdown
    assert "tools=0->1" in markdown


def test_network_diff_cli_writes_markdown_and_json(tmp_path: Path) -> None:
    direct_path = tmp_path / "direct.jsonl"
    cassandra_path = tmp_path / "cassandra.jsonl"
    markdown_path = tmp_path / "report.md"
    json_path = tmp_path / "report.json"
    record = {
        "method": "POST",
        "url": "https://api.anthropic.com/v1/messages",
        "request_headers": {},
        "request_body_b64": _body({"model": "claude"}),
        "response_status": 200,
    }
    _write_jsonl(direct_path, [record])
    _write_jsonl(cassandra_path, [record])

    result = CliRunner().invoke(
        main,
        [
            "capture",
            "network-diff",
            "--direct",
            str(direct_path),
            "--cassandra",
            str(cassandra_path),
            "--output",
            str(markdown_path),
            "--json-output",
            str(json_path),
        ],
    )

    assert result.exit_code == 0, result.output
    assert "Wrote Markdown report" in result.output
    assert "Differential Network Capture Report" in markdown_path.read_text(encoding="utf-8")
    payload = json.loads(json_path.read_text(encoding="utf-8"))
    assert payload["direct_count"] == 1
    assert payload["cassandra_count"] == 1
