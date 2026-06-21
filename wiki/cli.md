# CLI Reference

This page is the authoritative reference for the **Python Cassandra CLI** exposed by the `cassandra` console script.

## Global behavior

### Entry points

- Console script: `cassandra`
- Python module entrypoint: `python -m cassandra.cli`

### Global options

| Option | Scope | Meaning |
|---|---|---|
| `--help`, `-?` | root, groups, commands | Show help and exit |
| `--version`, `-v` | root only | Show the Cassandra version and exit |

> `-v` is a **root-level version alias**. Inside subcommands such as `cassandra wrap claude -v`, `-v` keeps its subcommand meaning (`--verbose`), not version.

## Command index

| Command | Purpose | Docker-native parity |
|---|---|---|
| `cassandra install ...` | Install and manage persistent deployments | **python-native; Docker-native wrapper supports `persistent-docker` lifecycle subset** |
| `cassandra proxy` | Run the Cassandra proxy server | **native in container** |
| `cassandra learn` | Learn from past tool-call failures | **native in container** |
| `cassandra perf` | Summarize recent proxy performance | **native in container** |
| `cassandra evals ...` | Run memory evaluation workflows | **native in container** |
| `cassandra memory ...` | Inspect and manage stored memories | **native in container** |
| `cassandra mcp ...` | Install, inspect, remove, or serve MCP integration | **native in container** |
| `cassandra wrap claude` | Start proxy and launch Claude Code | **host-bridged** |
| `cassandra wrap copilot` | Start proxy and launch GitHub Copilot CLI | **python-native only** |
| `cassandra wrap codex` | Start proxy and launch Codex CLI | **host-bridged** |
| `cassandra wrap aider` | Start proxy and launch Aider | **host-bridged** |
| `cassandra wrap cursor` | Start proxy and print Cursor config guidance | **host-bridged** |
| `cassandra wrap openclaw` | Install and configure the OpenClaw plugin | **host-bridged** |
| `cassandra unwrap openclaw` | Disable the Cassandra OpenClaw plugin | **host-bridged** |

## Captured `--help` output

The sections below capture the current top-level help output from the live CLI.

### `cassandra --help`

```text
Usage: cassandra [OPTIONS] COMMAND [ARGS]...

  Cassandra - The Context Optimization Layer for LLM Applications.

  Manage memories, run the optimization proxy, and analyze metrics.

  Examples:
      cassandra proxy              Start the optimization proxy
      cassandra memory list        List stored memories
      cassandra memory stats       Show memory statistics

Options:
  -v, --version  Show the version and exit.
  -?, --help     Show this message and exit.

Commands:
  evals   Memory evaluation commands.
  install Install and manage persistent Cassandra deployments.
  learn   Learn from past tool call failures to prevent future ones.
  mcp     MCP server for Claude Code integration.
  memory  Manage memories stored in Cassandra.
  perf    Analyze proxy performance from logs.
  proxy   Start the optimization proxy server.
  unwrap  Undo durable Cassandra wrapping for supported tools.
  wrap    Wrap CLI tools to run through Cassandra.
```

### Top-level command help snapshots

<details>
<summary><code>cassandra proxy --help</code></summary>

```text
Usage: cassandra proxy [OPTIONS]

  Start the optimization proxy server.

  Examples:
      cassandra proxy                    Start proxy on port 8787
      cassandra proxy --port 8080        Start proxy on port 8080
      cassandra proxy --no-optimize      Passthrough mode (no optimization)

  Usage with Claude Code:
      ANTHROPIC_BASE_URL=http://localhost:8787 claude

  Usage with OpenAI-compatible clients:
      OPENAI_BASE_URL=http://localhost:8787/v1 your-app
```

</details>

<details>
<summary><code>cassandra learn --help</code></summary>

```text
Usage: cassandra learn [OPTIONS]

  Learn from past tool call failures to prevent future ones.
```

</details>

<details>
<summary><code>cassandra perf --help</code></summary>

```text
Usage: cassandra perf [OPTIONS]

  Analyze proxy performance from logs.
```

</details>

<details>
<summary><code>cassandra evals --help</code></summary>

```text
Usage: cassandra evals [OPTIONS] COMMAND [ARGS]...

  Memory evaluation commands.

Commands:
  memory     Run LoCoMo memory evaluation benchmark.
  memory-v2  Run LoCoMo V2 evaluation with LLM-controlled memory tools.
```

</details>

<details>
<summary><code>cassandra memory --help</code></summary>

```text
Usage: cassandra memory [OPTIONS] COMMAND [ARGS]...

  Manage memories stored in Cassandra.

Commands:
  delete  Delete one or more memories by ID.
  edit    Edit a memory's content or importance.
  export  Export all memories to JSON.
  import  Import memories from a JSON file.
  list    List stored memories with optional filters.
  prune   Prune memories matching specified criteria.
  purge   Delete ALL memories from the database.
  show    Show full details of a single memory.
  stats   Show memory store statistics.
```

</details>

<details>
<summary><code>cassandra mcp --help</code></summary>

```text
Usage: cassandra mcp [OPTIONS] COMMAND [ARGS]...

  MCP server for Claude Code integration.

Commands:
  install    Install Cassandra MCP server into Claude Code config.
  serve      Start the MCP server (called by Claude Code).
  status     Check Cassandra MCP configuration status.
  uninstall  Remove Cassandra MCP server from Claude Code config.
```

</details>

<details>
<summary><code>cassandra install --help</code></summary>

```text
Usage: cassandra install [OPTIONS] COMMAND [ARGS]...

  Install and manage persistent Cassandra deployments.

Options:
  -?, --help  Show this message and exit.

Commands:
  apply    Install a persistent Cassandra deployment.
  remove   Remove a persistent deployment and undo managed config.
  restart  Restart a persistent deployment.
  start    Start a persistent deployment.
  status   Show persistent deployment status.
  stop     Stop a persistent deployment.
```

</details>

<details>
<summary><code>cassandra wrap --help</code></summary>

```text
Usage: cassandra wrap [OPTIONS] COMMAND [ARGS]...

  Wrap CLI tools to run through Cassandra.

Commands:
  aider     Launch aider through Cassandra proxy.
  claude    Launch Claude Code through Cassandra proxy.
  copilot   Launch GitHub Copilot CLI through Cassandra proxy.
  codex     Launch OpenAI Codex CLI through Cassandra proxy.
  cursor    Start Cassandra proxy for use with Cursor.
  openclaw  Install and configure Cassandra OpenClaw plugin in one command.
```

</details>

<details>
<summary><code>cassandra unwrap --help</code></summary>

```text
Usage: cassandra unwrap [OPTIONS] COMMAND [ARGS]...

  Undo durable Cassandra wrapping for supported tools.

Commands:
  openclaw  Disable the Cassandra OpenClaw plugin and restore the legacy engine slot.
```

</details>

## `cassandra proxy`

Start the optimization proxy server.

```bash
cassandra proxy
cassandra proxy --port 8787
cassandra proxy --mode cache
```

| Option | Default | Meaning |
|---|---|---|
| `--host` | `127.0.0.1` | Host interface to bind |
| `--port`, `-p` | `8787` | Port to bind |
| `--mode` | runtime default | Optimization mode: `token`, `cache`, `token_mode`, `cache_mode`, `token_savings`, `cost_savings`, `token_cassandra` |
| `--no-optimize` | off | Disable optimization and operate in passthrough mode |
| `--no-cache` | off | Disable semantic caching |
| `--no-rate-limit` | off | Disable rate limiting |
| `--retry-max-attempts` | runtime default `3` | Maximum upstream retry attempts |
| `--request-timeout-seconds` | runtime default `300` | Request timeout in seconds |
| `--connect-timeout-seconds` | runtime default `10` | Upstream connection timeout |
| `--anthropic-pre-upstream-concurrency` | auto `max(2, min(8, cpu_count))` | Cap simultaneous pre-upstream work on `/v1/messages` (body read, deep copy, first compression stage, memory-context lookup, upstream connect). `0` or negative disables (unbounded); any positive integer is honoured verbatim. Prevents cold-start replay storms from starving `/livez`, `/readyz`, and new Codex WS opens. |
| `--anthropic-pre-upstream-acquire-timeout-seconds` | `15.0` | Fail fast when the Anthropic pre-upstream queue is saturated. Requests that wait longer return `503` with `Retry-After` instead of parking indefinitely. |
| `--anthropic-pre-upstream-memory-context-timeout-seconds` | `2.0` | Fail-open timeout for Anthropic memory-context lookup while the request still holds a pre-upstream slot. |
| `--log-file` | unset | JSONL log output path |
| `--budget` | unset | Daily USD budget limit |
| `--no-code-aware` | off | Disable AST-aware code compression |
| `--code-aware` | off | Enable code-aware compression in the proxy (env: CASSANDRA_CODE_AWARE_ENABLED) |
| `--no-read-lifecycle` | off | Disable stale/superseded read compression |
| `--no-ccr-inject-tool` | off | Disable injecting the `cassandra_retrieve` tool |
| `--no-ccr-marker` | off | Disable adding retrieval markers to compressed output |
| `--no-ccr-proactive-expansion` | off | Disable proactive CCR context expansion |
| `--memory` | off | Enable persistent user memory |
| `--memory-db-path` | `""` | Override memory DB path (help text: `{cwd}/.cassandra/memory.db`) |
| `--no-memory-tools` | off | Disable automatic memory tool injection |
| `--no-memory-context` | off | Disable automatic memory context injection |
| `--memory-top-k` | `10` | Number of memories to inject |
| `--learn` | off | Enable live traffic learning |
| `--no-learn` | off | Explicitly disable traffic learning |
| `--backend` | `anthropic` | Backend: `anthropic`, `bedrock`, `openrouter`, `anyllm`, or `litellm-*` |
| `--anyllm-provider` | `openai` | Provider name for `anyllm` |
| `--anthropic-api-url` | unset | Custom Anthropic passthrough API URL |
| `--openai-api-url` | unset | Custom OpenAI passthrough API URL |
| `--gemini-api-url` | unset | Custom Gemini passthrough API URL |
| `--region` | `us-west-2` | Cloud region for Bedrock / Vertex / related backends |
| `--bedrock-region` | unset | Deprecated Bedrock region override |
| `--bedrock-profile` | unset | AWS profile name for Bedrock |
| `--telemetry` | off | Opt in to anonymous usage telemetry (off by default) |
| `--no-telemetry` | off | Force anonymous usage telemetry off (already the default) |

Notes:

- `--learn` implies memory unless `--no-learn` is also set.
- Proxy startup can also read environment variables such as `CASSANDRA_HOST`, `CASSANDRA_PORT`, `CASSANDRA_BUDGET`, `CASSANDRA_MODE`, `CASSANDRA_ANYLLM_PROVIDER`, `CASSANDRA_ANTHROPIC_PRE_UPSTREAM_CONCURRENCY`, `CASSANDRA_ANTHROPIC_PRE_UPSTREAM_ACQUIRE_TIMEOUT_SECONDS`, `CASSANDRA_REQUEST_TIMEOUT`, `CASSANDRA_ANTHROPIC_PRE_UPSTREAM_MEMORY_CONTEXT_TIMEOUT_SECONDS`, `ANTHROPIC_TARGET_API_URL`, `OPENAI_TARGET_API_URL`, and `GEMINI_TARGET_API_URL`. CLI flags take precedence over environment variables.
- The default Anthropic pre-upstream cap is intentionally conservative for CPU/ONNX-heavy work. Larger containers may want to raise it after checking the resolved runtime values on `/readyz` or `/debug/warmup`.

See also: [Proxy Server](proxy.md), [Configuration](configuration.md)

## `cassandra learn`

Learn from past tool-call failures and produce agent guidance.

```bash
cassandra learn
cassandra learn --apply
cassandra learn --agent codex --all
```

| Option | Default | Meaning |
|---|---|---|
| `--project` | current project resolution | Target project path |
| `--all` | off | Analyze all discovered projects |
| `--apply` | off | Write recommendations instead of dry-run output |
| `--agent` | `auto` | Agent source: `auto`, built-ins (`claude`, `codex`, `gemini`), or plugin-provided names |
| `--model` | auto-detect | LLM model used for analysis |

Notes:

- `--agent auto` scans all detected agent data sources.
- If `--project` is omitted, Cassandra resolves from the current directory upward.
- External agent integrations register through the `cassandra.learn_plugin` entry point.

See also: [Failure Learning](learn.md)

## `cassandra perf`

Summarize recent proxy performance from the local proxy log.

```bash
cassandra perf
cassandra perf --hours 24
cassandra perf --raw
```

| Option | Default | Meaning |
|---|---|---|
| `--hours` | `168.0` | Time window in hours |
| `--raw` | off | Print raw PERF records instead of the summarized report |

The command reads `${CASSANDRA_WORKSPACE_DIR}/logs/proxy.log` (defaults
to `~/.cassandra/logs/proxy.log` — see the
[Filesystem Contract](filesystem-contract.md)).

## `cassandra evals`

Memory evaluation command group.

### `cassandra evals memory`

Run the LoCoMo memory evaluation benchmark.

```bash
cassandra evals memory -n 3
cassandra evals memory --answer-model gpt-4o --llm-judge
```

| Option | Default | Meaning |
|---|---|---|
| `--n-conversations`, `-n` | all available | Number of conversations to evaluate |
| `--categories` | benchmark default | Comma-separated categories |
| `--include-adversarial` | off | Include category 5 / unanswerable questions |
| `--top-k` | `10` | Memories retrieved per question |
| `--f1-threshold` | `0.5` | Threshold for correctness |
| `--answer-model` | unset | Model for answer generation |
| `--llm-judge` | off | Use LLM-as-judge scoring |
| `--judge-provider` | `litellm` | Judge provider: `openai`, `anthropic`, `litellm`, `simple` |
| `--judge-model` | `gpt-4o` | Judge model |
| `--output`, `-o` | unset | Save JSON results to a path |
| `--no-extract` | off | Disable LLM memory extraction |
| `--extraction-model` | `gpt-4o-mini` | Memory extraction model |
| `--pass-all` | off | Require all checks to pass |
| `--parallel` | `10` | Parallel worker count |
| `--debug` | off | Enable debug output |

### `cassandra evals memory-v2`

Run the V2 memory evaluation flow with LLM-controlled tools.

```bash
cassandra evals memory-v2
cassandra evals memory-v2 --save-model gpt-4o-mini --llm-judge
```

| Option | Default | Meaning |
|---|---|---|
| `--n-conversations`, `-n` | all available | Number of conversations to evaluate |
| `--categories` | benchmark default | Comma-separated categories |
| `--include-adversarial` | off | Include adversarial questions |
| `--f1-threshold` | `0.5` | Threshold for correctness |
| `--save-model` | `gpt-4o-mini` | Model used when persisting memories |
| `--answer-model` | `gpt-4o` | Answer model |
| `--max-results` | `10` | Maximum tool results |
| `--no-graph` | off | Disable graph usage |
| `--llm-judge` | off | Use LLM-as-judge scoring |
| `--judge-model` | `gpt-4o` | Judge model |
| `--output`, `-o` | unset | Save JSON results |
| `--parallel` | `5` | Parallel worker count |
| `--debug` | off | Enable debug output |

Hidden compatibility shims exist for older command paths:

- `cassandra memory-eval`
- `cassandra memory-eval-v2`

These are intentionally omitted from normal usage docs.

## `cassandra memory`

Memory management command group. This group is only registered when the optional memory dependencies import successfully.

### `cassandra memory list`

```bash
cassandra memory list
cassandra memory list --scope USER --since 7d
cassandra memory list -q "budget"
```

| Option | Default | Meaning |
|---|---|---|
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--limit`, `-n` | `50` | Maximum memories to show |
| `--session`, `-s` | unset | Filter by session ID |
| `--scope` | unset | `USER`, `SESSION`, `AGENT`, or `TURN` |
| `--since` | unset | Age filter using duration syntax such as `7d`, `2w`, `1m` |
| `--search`, `-q` | unset | Content search query |

### `cassandra memory show <memory_id>`

```bash
cassandra memory show 1234abcd
cassandra memory show 1234abcd --json
```

| Argument / option | Default | Meaning |
|---|---|---|
| `memory_id` | required | Full or partial memory ID |
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--json` | off | Emit raw JSON |

### `cassandra memory stats`

```bash
cassandra memory stats
```

| Option | Default | Meaning |
|---|---|---|
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |

### `cassandra memory edit <memory_id>`

```bash
cassandra memory edit 1234abcd --content "Updated note"
cassandra memory edit 1234abcd --importance 0.9
```

| Argument / option | Default | Meaning |
|---|---|---|
| `memory_id` | required | Full or partial memory ID |
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--content`, `-c` | unset | New memory content |
| `--importance`, `-i` | unset | New importance score (`0.0` to `1.0`) |

At least one of `--content` or `--importance` is required.

### `cassandra memory delete <memory_ids...>`

```bash
cassandra memory delete 1234abcd 5678efgh
cassandra memory delete 1234abcd --force
```

| Argument / option | Default | Meaning |
|---|---|---|
| `memory_ids...` | required | One or more memory IDs |
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--force`, `-f` | off | Skip confirmation |

### `cassandra memory prune`

```bash
cassandra memory prune --older-than 30d --dry-run
cassandra memory prune --scope SESSION --force
```

| Option | Default | Meaning |
|---|---|---|
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--older-than` | unset | Age threshold |
| `--scope` | unset | Scope filter: `USER`, `SESSION`, `AGENT`, `TURN` |
| `--low-importance` | unset | Importance cutoff |
| `--session`, `-s` | unset | Session ID filter |
| `--dry-run` | off | Show what would be removed |
| `--force`, `-f` | off | Skip confirmation |

At least one filter is required. Filters combine with **AND** semantics.

### `cassandra memory purge`

```bash
cassandra memory purge --confirm
```

| Option | Default | Meaning |
|---|---|---|
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--confirm` | off | Required confirmation flag |

### `cassandra memory export`

```bash
cassandra memory export
cassandra memory export --output export.json
```

| Option | Default | Meaning |
|---|---|---|
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--output`, `-o` | stdout | Output path |

### `cassandra memory import <file>`

```bash
cassandra memory import export.json
cassandra memory import export.json --force
```

| Argument / option | Default | Meaning |
|---|---|---|
| `file` | required | JSON file containing exported memories |
| `--db-path` | `./.cassandra/memory.db` if present, else `~/.cassandra/memory.db` | Memory database path |
| `--force`, `-f` | off | Skip confirmation |

The import expects a JSON array. Malformed entries are skipped.

## `cassandra mcp`

Manage the Cassandra MCP server integration.

### `cassandra mcp install`

```bash
cassandra mcp install
cassandra mcp install --proxy-url http://127.0.0.1:9000
```

| Option | Default | Meaning |
|---|---|---|
| `--proxy-url` | `http://127.0.0.1:8787` | Proxy URL written into MCP config |
| `--force` | off | Overwrite an existing Cassandra MCP config |

### `cassandra mcp uninstall`

```bash
cassandra mcp uninstall
```

This removes the Cassandra MCP server entry from the Claude configuration.

### `cassandra mcp status`

```bash
cassandra mcp status
```

This inspects MCP SDK availability, Claude config state, and proxy reachability.

### `cassandra mcp serve`

```bash
cassandra mcp serve
cassandra mcp serve --proxy-url http://127.0.0.1:9000 --debug
```

| Option | Default | Meaning |
|---|---|---|
| `--proxy-url` | `http://127.0.0.1:8787` | Proxy URL (also reads `CASSANDRA_PROXY_URL`) |
| `--direct` | off | Disable stdio transport wrapping |
| `--debug` | off | Enable debug logging |

`serve` is part of the public CLI, but it is usually consumed by MCP host tooling rather than by humans directly.

See also: [MCP Tools](mcp.md)

## `cassandra install`

Install and manage persistent local Cassandra deployments.

### `cassandra install apply --help`

```text
Usage: cassandra install apply [OPTIONS]

  Install a persistent Cassandra deployment.

Options:
  --preset [persistent-service|persistent-task|persistent-docker]
                                  Persistent runtime preset to install.
                                  [default: persistent-service]
  --runtime [python|docker]       Runtime used to execute Cassandra for
                                  service/task modes.  [default: python]
  --scope [provider|user|system]  Where to apply persistent configuration.
                                  [default: user]
  --providers [auto|all|manual]   Target selection mode for direct tool
                                  configuration.  [default: auto]
  --target [claude|copilot|codex|aider|cursor|openclaw]
                                  Tool target to configure when --providers
                                  manual is used.
  --profile TEXT                  Deployment profile name.  [default: default]
  -p, --port INTEGER              Persistent proxy port.  [default: 8787]
  --backend TEXT                  Proxy backend for the persistent runtime.
                                  [default: anthropic]
  --anyllm-provider TEXT          Provider for any-llm backends when --backend
                                  anyllm is used.
  --region TEXT                   Cloud region for Bedrock / Vertex style
                                  backends.
  --mode TEXT                     Proxy optimization mode.  [default: token]
  --memory                        Enable persistent memory in the proxy runtime.
  --telemetry                     Opt in to anonymous telemetry in the runtime
                                  (off by default).
  --no-telemetry                  Force anonymous telemetry off in the runtime
                                  (already the default).
  --image TEXT                    Docker image to use when runtime=docker or
                                  preset=persistent-docker.  [default:
                                  ghcr.io/Sarthak-47/cassandra:latest]
  -?, --help                      Show this message and exit.
```

### `cassandra install apply`

```bash
cassandra install apply --preset persistent-service --providers auto
cassandra install apply --preset persistent-task --providers manual --target claude --target codex
cassandra install apply --preset persistent-docker --scope user
```

| Option | Default | Meaning |
|---|---|---|
| `--preset` | `persistent-service` | Lifecycle preset: `persistent-service`, `persistent-task`, or `persistent-docker` |
| `--runtime` | `python` | Runtime used for service/task installs: `python` or `docker` |
| `--scope` | `user` | Config scope: `provider`, `user`, or `system` |
| `--providers` | `auto` | Target selection mode: `auto`, `all`, or `manual` |
| `--target` | repeatable | Tool target used with `--providers manual` |
| `--profile` | `default` | Deployment profile name |
| `--port`, `-p` | `8787` | Persistent proxy port |
| `--backend` | `anthropic` | Backend for the managed runtime |
| `--anyllm-provider` | unset | Provider name used with `--backend anyllm` |
| `--region` | unset | Cloud region override |
| `--mode` | `token` | Proxy optimization mode |
| `--memory` | off | Enable persistent memory in the managed runtime |
| `--telemetry` | off | Opt in to anonymous telemetry (off by default) |
| `--no-telemetry` | off | Force anonymous telemetry off (already the default) |
| `--image` | `ghcr.io/Sarthak-47/cassandra:latest` | Docker image for Docker-backed installs |

`apply` stores a manifest under
`${CASSANDRA_WORKSPACE_DIR}/deploy/<profile>/manifest.json` (default
`~/.cassandra/deploy/<profile>/manifest.json`), applies managed tool
configuration, starts the chosen runtime, and waits for `readyz`.

Docker-native host wrappers expose a narrower `cassandra install` subset for `persistent-docker` only: `apply`, `status`, `start`, `stop`, `restart`, and `remove`. Those wrapper flows preserve the same port and manifest behavior, but they intentionally reject `persistent-service`, `persistent-task`, and provider mutation flags like `--scope`, `--providers`, and `--target`.

### `cassandra install status`

```bash
cassandra install status
cassandra install status --profile default
```

Shows the stored profile, preset, runtime, supervisor kind, scope, port, runtime status, readiness, and backend from `/health`.

### `cassandra install start`

```bash
cassandra install start
cassandra install start --profile default
```

Starts a previously installed deployment profile without reapplying mutations.

### `cassandra install stop`

```bash
cassandra install stop
```

Stops the managed runtime for an installed deployment profile.

### `cassandra install restart`

```bash
cassandra install restart
```

Stops and starts the selected deployment profile.

### `cassandra install remove`

```bash
cassandra install remove
```

Stops the runtime, removes installed supervisor artifacts, reverts managed configuration changes, and deletes the stored manifest.

See also: [Persistent Installs](persistent-installs.md)

## `cassandra wrap`

Wrap external coding tools so their traffic flows through Cassandra.

### Shared semantics

- `--port`, when available, defaults to `8787`
- `--no-proxy` skips proxy startup and assumes an existing proxy
- `--learn` enables live traffic learning
- `-v`, `--verbose` means **verbose output**
- Hidden `--prepare-only` exists for internal Docker-native bridge flows and is intentionally omitted from normal usage

### `cassandra wrap claude`

```bash
cassandra wrap claude
cassandra wrap claude --resume <session-id>
cassandra wrap claude --port 9999
```

| Option / arg | Default | Meaning |
|---|---|---|
| `--port`, `-p` | `8787` | Proxy port |
| `--no-rtk` | off | Skip `rtk` installation and hook registration |
| `--no-proxy` | off | Reuse an existing proxy |
| `--learn` | off | Enable live traffic learning |
| `--verbose`, `-v` | off | Verbose output |
| `claude_args...` | passthrough | Additional Claude Code arguments |

Requires the `claude` binary on the host.

### `cassandra wrap codex`

```bash
cassandra wrap codex
cassandra wrap codex -- "fix the bug"
cassandra wrap codex --backend anyllm --anyllm-provider groq
```

| Option / arg | Default | Meaning |
|---|---|---|
| `--port`, `-p` | `8787` | Proxy port |
| `--no-rtk` | off | Skip `rtk` installation and `AGENTS.md` injection |
| `--no-proxy` | off | Reuse an existing proxy |
| `--learn` | off | Enable live traffic learning |
| `--backend` | unset | Proxy backend override |
| `--anyllm-provider` | unset | `anyllm` provider override |
| `--region` | unset | Cloud region override |
| `--verbose`, `-v` | off | Verbose output |
| `codex_args...` | passthrough | Additional Codex CLI arguments |

Requires the `codex` binary on the host.

### `cassandra wrap copilot`

```bash
cassandra wrap copilot -- --model claude-sonnet-4-20250514
cassandra wrap copilot --backend anyllm --anyllm-provider groq -- --model gpt-4o
```

| Option / arg | Default | Meaning |
|---|---|---|
| `--port`, `-p` | `8787` | Proxy port |
| `--no-rtk` | off | Skip `rtk` installation and GitHub Copilot instructions injection |
| `--no-proxy` | off | Reuse an existing proxy |
| `--learn` | off | Enable live traffic learning |
| `--backend` | unset | Proxy backend override |
| `--anyllm-provider` | unset | `anyllm` provider override |
| `--region` | unset | Cloud region override |
| `--provider-type` | `auto` | Force Copilot BYOK provider type (`anthropic` or `openai`) |
| `--wire-api` | unset | OpenAI wire API override for OpenAI-style backends |
| `--verbose`, `-v` | off | Verbose output |
| `copilot_args...` | passthrough | Additional Copilot CLI arguments |

Requires the `copilot` binary on the host. When a matching persistent deployment exists on the requested port, `wrap copilot` reuses or recovers it before falling back to an ephemeral proxy.

### `cassandra wrap aider`

```bash
cassandra wrap aider
cassandra wrap aider -- --model gpt-4o
cassandra wrap aider --backend litellm-vertex --region us-central1
```

| Option / arg | Default | Meaning |
|---|---|---|
| `--port`, `-p` | `8787` | Proxy port |
| `--no-rtk` | off | Skip `rtk` installation and `CONVENTIONS.md` injection |
| `--no-proxy` | off | Reuse an existing proxy |
| `--learn` | off | Enable live traffic learning |
| `--backend` | unset | Proxy backend override |
| `--anyllm-provider` | unset | `anyllm` provider override |
| `--region` | unset | Cloud region override |
| `--verbose`, `-v` | off | Verbose output |
| `aider_args...` | passthrough | Additional Aider arguments |

Requires the `aider` binary on the host.

### `cassandra wrap cursor`

```bash
cassandra wrap cursor
cassandra wrap cursor --port 9999
cassandra wrap cursor --no-rtk
```

| Option | Default | Meaning |
|---|---|---|
| `--port`, `-p` | `8787` | Proxy port |
| `--no-rtk` | off | Skip `rtk` installation and `.cursorrules` injection |
| `--no-proxy` | off | Reuse an existing proxy |
| `--learn` | off | Enable live traffic learning |
| `--verbose`, `-v` | off | Verbose output |

This command prints Cursor configuration instructions and waits while the proxy stays up. It does **not** launch Cursor directly.

### `cassandra wrap openclaw`

```bash
cassandra wrap openclaw
cassandra wrap openclaw --plugin-path ./plugins/openclaw
```

| Option | Default | Meaning |
|---|---|---|
| `--plugin-path` | unset | Local plugin source directory |
| `--plugin-spec` | `cassandra-ai/openclaw` | NPM plugin spec |
| `--skip-build` | off | Skip local `npm install` / build steps |
| `--copy` | off | Copy plugin instead of linked install |
| `--proxy-port` | `8787` | Cassandra proxy port |
| `--startup-timeout-ms` | `20000` | Proxy startup timeout |
| `--gateway-provider-id` | repeatable | OpenClaw provider IDs routed through Cassandra |
| `--python-path` | unset | Python launcher override |
| `--no-auto-start` | off | Disable plugin auto-start behavior |
| `--no-restart` | off | Do not restart the OpenClaw gateway |
| `--verbose`, `-v` | off | Verbose output |

Requires the `openclaw` binary on the host, and local-source mode may also require `npm`. In Docker-native mode, the installed host wrapper drives the host `openclaw` CLI while the plugin auto-starts the host `cassandra` wrapper from `PATH`.

## `cassandra unwrap`

Undo durable wrapping for supported tools.

### `cassandra unwrap openclaw`

```bash
cassandra unwrap openclaw
cassandra unwrap openclaw --no-restart
```

| Option | Default | Meaning |
|---|---|---|
| `--no-restart` | off | Do not restart the OpenClaw gateway |
| `--verbose`, `-v` | off | Verbose output |

This disables the Cassandra OpenClaw plugin and restores the legacy context engine slot.

## Docker-native parity matrix

This matrix compares the **Python CLI contract** to the Docker-native host wrapper added in this branch.

Legend:

- **native in container** — the command runs entirely inside the Cassandra container
- **host-bridged** — Cassandra runs in Docker, but the wrapped external tool still runs on the host

| Command path | Python CLI | Docker-native wrapper | Parity |
|---|---|---|---|
| `cassandra proxy` | native | native in container | full |
| `cassandra learn` | native | native in container | full |
| `cassandra perf` | native | native in container | full |
| `cassandra evals memory` | native | native in container | full |
| `cassandra evals memory-v2` | native | native in container | full |
| `cassandra memory ...` | native (when memory deps are available) | native in container | full |
| `cassandra mcp install` | native | native in container | full |
| `cassandra mcp uninstall` | native | native in container | full |
| `cassandra mcp status` | native | native in container | full |
| `cassandra mcp serve` | native | native in container | full |
| `cassandra install apply|status|start|stop|restart|remove` | native | Docker-native wrapper for `persistent-docker`; compose remains an alternative | partial |
| `cassandra wrap claude` | native | host-bridged | partial |
| `cassandra wrap copilot` | native | not implemented in Docker-native wrapper | none |
| `cassandra wrap codex` | native | host-bridged | partial |
| `cassandra wrap aider` | native | host-bridged | partial |
| `cassandra wrap cursor` | native | host-bridged | partial |
| `cassandra wrap openclaw` | native | host-bridged | partial |
| `cassandra unwrap openclaw` | native | host-bridged | partial |

For the Docker-native execution model itself, see [Docker-Native Install](docker-install.md). For persistent service/task/docker lifecycle management, see [Persistent Installs](persistent-installs.md).

## Hidden and compatibility-only command paths

These exist in code but are intentionally excluded from normal user docs:

- `cassandra memory-eval`
- `cassandra memory-eval-v2`
- hidden internal `--prepare-only` flags on `wrap` subcommands

If you are documenting operational behavior or debugging internal wrapper flows, refer to the implementation in `cassandra/cli/wrap.py`.
