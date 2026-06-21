# Filesystem Contract

Cassandra writes configuration, runtime state, logs, and caches to a small
set of well-known paths under the user's home directory. This page is the
source of truth for where those paths live, how to override them, and how
they behave inside Docker containers.

## Two-root model

| Variable | Default | Purpose | Typical access |
|---|---|---|---|
| `CASSANDRA_CONFIG_DIR` | `~/.cassandra/config` | User/admin-authored configuration (model catalogs, plugin settings, etc.) | Read-mostly |
| `CASSANDRA_WORKSPACE_DIR` | `~/.cassandra` | Runtime state written by the proxy and CLI (savings, logs, memory DB, telemetry, caches) | Read-write |

Both variables are recognized by the Python proxy / CLI and the npm SDK.
They are **additive** — every pre-existing per-resource env var
(`CASSANDRA_SAVINGS_PATH`, `CASSANDRA_TOIN_PATH`,
`CASSANDRA_SUBSCRIPTION_STATE_PATH`, `CASSANDRA_MODEL_LIMITS`, ...)
continues to work with identical semantics.

## Precedence

For every per-resource helper, resolution follows this order:

```
explicit argument
    │ falls through when None/""
    ▼
per-resource env var (e.g. CASSANDRA_SAVINGS_PATH)
    │ falls through when unset/blank
    ▼
derived from canonical root
    │ e.g. ${CASSANDRA_WORKSPACE_DIR}/proxy_savings.json
    ▼
default (e.g. ~/.cassandra/proxy_savings.json)
```

Examples:

- `CASSANDRA_WORKSPACE_DIR=/mnt/state` → savings land at
  `/mnt/state/proxy_savings.json` unless `CASSANDRA_SAVINGS_PATH` overrides.
- `CASSANDRA_SAVINGS_PATH=/custom/savings.json` always wins, even when
  `CASSANDRA_WORKSPACE_DIR` is set.
- Unset both and the default is `~/.cassandra/proxy_savings.json`.

## Bucket assignments

### Workspace bucket (`CASSANDRA_WORKSPACE_DIR`)

| Resource | Default path | Legacy env var |
|---|---|---|
| Proxy savings ledger | `${WORKSPACE_DIR}/proxy_savings.json` | `CASSANDRA_SAVINGS_PATH` |
| TOIN telemetry JSON | `${WORKSPACE_DIR}/toin.json` | `CASSANDRA_TOIN_PATH` |
| Subscription tracker state | `${WORKSPACE_DIR}/subscription_state.json` | `CASSANDRA_SUBSCRIPTION_STATE_PATH` |
| Memory SQLite | `${WORKSPACE_DIR}/memory.db` | CLI `--memory-db-path` |
| Native memory directory | `${WORKSPACE_DIR}/memories/` | `MemoryConfig.native_memory_dir` |
| License cache | `${WORKSPACE_DIR}/license_cache.json` | — |
| Session stats JSONL | `${WORKSPACE_DIR}/session_stats.jsonl` | — |
| Memory sync state | `${WORKSPACE_DIR}/sync_state.json` | — |
| Memory bridge state | `${WORKSPACE_DIR}/bridge_state.json` | — |
| Proxy log directory | `${WORKSPACE_DIR}/logs/` | — |
| HTTP 400 debug dumps | `${WORKSPACE_DIR}/logs/debug_400/` | — |
| Vendored `rtk` binary | `${WORKSPACE_DIR}/bin/rtk[.exe]` | — |
| Deployment profiles | `${WORKSPACE_DIR}/deploy/` | — |
| Beacon lock file | `${WORKSPACE_DIR}/.beacon_lock_<port>` | — |

### Config bucket (`CASSANDRA_CONFIG_DIR`)

| Resource | Default path | Legacy env var |
|---|---|---|
| Models catalog | `${CONFIG_DIR}/models.json` | `CASSANDRA_MODEL_LIMITS` (content override) |
| Plugin settings | `${CONFIG_DIR}/plugins/<name>/...` | — |

### Backward compatibility — models.json

`models.json` historically lived at `~/.cassandra/models.json` (i.e. in the
workspace root, not in `config/`). For a seamless migration the Python
providers check **both** locations in this order:

1. `${CASSANDRA_CONFIG_DIR}/models.json` (new canonical location)
2. `${CASSANDRA_WORKSPACE_DIR}/models.json` (legacy fallback)

Existing installs continue to work unchanged. New installs are encouraged
to put `models.json` in the config bucket.

## Plugin authors

Two helpers give plugins isolated, per-plugin directories under both
roots:

### Python

```python
from cassandra import paths

cfg_dir = paths.plugin_config_dir("my-plugin")
# → ~/.cassandra/config/plugins/my-plugin

state_dir = paths.plugin_workspace_dir("my-plugin")
# → ~/.cassandra/plugins/my-plugin

cfg_dir.mkdir(parents=True, exist_ok=True)
(cfg_dir / "settings.json").write_text("{}")
```

### npm SDK

```typescript
import { pluginConfigDir, pluginWorkspaceDir } from "@cassandra/sdk";

const cfgDir = pluginConfigDir("my-plugin");
const stateDir = pluginWorkspaceDir("my-plugin");
```

Plugin-author helpers reject names containing `/` or `\` to keep the
namespace flat.

## Docker naming overlap: `CASSANDRA_WORKSPACE` vs `CASSANDRA_WORKSPACE_DIR`

These are **two different variables** with different semantics, both
retained for backward compatibility:

| Variable | Scope | Meaning |
|---|---|---|
| `CASSANDRA_WORKSPACE` | Host-side (Docker) | Directory to bind-mount into the container as `/workspace` (equivalent to CWD in native runs). Used by `docker-compose.native.yml`. |
| `CASSANDRA_WORKSPACE_DIR` | Inside-the-container | Canonical Cassandra state root. Resolves to `/tmp/cassandra-home/.cassandra` inside the official container image, which in turn bind-mounts to `${HOME}/.cassandra` on the host. |

The official Docker bootstrap (compose file, `scripts/install.sh`, and the
Python `install` command) sets `CASSANDRA_WORKSPACE_DIR` and
`CASSANDRA_CONFIG_DIR` inside the container so the proxy resolves state to
the bind-mounted path without any user action.

## Project-scoped `.cassandra/` directories

A few code paths deliberately use **project-local** `.cassandra/` paths
resolved relative to the current working directory rather than the
canonical workspace root:

- `cassandra/proxy/server.py` — project-scoped memory DB default
- `cassandra/memory/mcp_server.py` — project-scoped memory DB default
- `cassandra/cli/wrap.py` — project-scoped memory and hook artifacts

These **do not obey** `CASSANDRA_WORKSPACE_DIR`. This is intentional: it
preserves the "project memory lives in the project directory" invariant
documented in [memory.md](memory.md). Users who want a single centrally
located memory store can pass `--memory-db-path <path>` explicitly or set
the path via the plugin API.

## Legacy per-resource env vars

Every legacy env var continues to work with its original semantics (raw
string in, raw string out — no tilde expansion, no path-separator
normalization), ensuring byte-for-byte backward compatibility.

Full list:

- `CASSANDRA_SAVINGS_PATH`
- `CASSANDRA_TOIN_PATH`
- `CASSANDRA_SUBSCRIPTION_STATE_PATH`
- `CASSANDRA_MODEL_LIMITS` (content override — JSON string or file path)

## See also

- [configuration.md](configuration.md) — general configuration reference
- [docker-install.md](docker-install.md) — Docker install details
- [persistent-installs.md](persistent-installs.md) — persistent
  deployment profiles
- [memory.md](memory.md) — memory-system paths and project scoping
