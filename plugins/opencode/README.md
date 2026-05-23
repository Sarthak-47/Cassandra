# cassandra-opencode

OpenCode integration helpers for Cassandra. The package supports two integration paths:

1. Provider config helpers used by `cassandra wrap opencode` and persistent installs.
2. A native OpenCode plugin that installs Cassandra transport interception and exposes the retrieve tool.

## Install

```bash
npm install cassandra-opencode
```

## Provider Config Helpers

Use these helpers when you need to generate OpenCode config that routes a `cassandra` provider through a running Cassandra proxy.

```ts
import {
  buildOpencodeConfigContent,
  createCassandraProvider,
} from "cassandra-opencode";

const provider = createCassandraProvider({ proxyPort: 8787 });
const config = buildOpencodeConfigContent({
  proxyPort: 8787,
  defaultModel: "claude-sonnet-4-6",
});

console.log(provider.provider.cassandra.npm);
console.log(config.model);
```

The generated provider uses `@ai-sdk/openai-compatible` and points model requests at `http://127.0.0.1:<port>/v1`.

## Native OpenCode Plugin

Use `CassandraPlugin` when OpenCode should intercept provider traffic in-process and expose Cassandra tooling from a plugin.

```ts
import { CassandraPlugin } from "cassandra-opencode";

export default async function plugin(input) {
  return CassandraPlugin(input, {
    proxyUrl: process.env.CASSANDRA_PROXY_URL ?? "http://127.0.0.1:8787",
  });
}
```

`CassandraPlugin`:

- installs Cassandra transport interception for OpenCode provider traffic.
- exposes the `cassandra_retrieve` tool.
- publishes `CASSANDRA_PROXY_URL` in the plugin output env.
- defaults to `http://127.0.0.1:8787` when no proxy URL is supplied.

## Retrieve Tool

```ts
import { createCassandraRetrieveTool } from "cassandra-opencode";

const retrieve = createCassandraRetrieveTool({
  proxyBaseUrl: "http://127.0.0.1:8787",
});

const result = await retrieve.execute({
  hash: "0123456789abcdef01234567",
});
```

The tool calls `/v1/retrieve/<hash>` on the Cassandra proxy.

## Compression Helper

```ts
import { compressWithCassandra } from "cassandra-opencode";

const result = await compressWithCassandra(
  [{ role: "user", content: "Summarize this file" }],
  { model: "gpt-4o", proxyUrl: "http://127.0.0.1:8787" },
);

console.log(`Saved ${result.tokensSaved} tokens`);
```

## Models

| Model | Context | Output |
|---|---:|---:|
| `claude-sonnet-4-6` | 200K | 16K |
| `claude-opus-4-6` | 200K | 16K |
| `claude-haiku-4-5-20251001` | 200K | 8K |
| `gpt-4o` | 128K | 16K |
| `gpt-4.1` | 1M | 32K |

The provider config exposes these as `cassandra/<model>` and defaults to `cassandra/claude-sonnet-4-6`.

## Environment

| Variable | Used by | Description |
|---|---|---|
| `CASSANDRA_PROXY_URL` | Native plugin | Proxy URL used by `CassandraPlugin` |
| `OPENCODE_CONFIG_CONTENT` | OpenCode wrapper | Generated OpenCode provider, model, and MCP config |

## License

Apache-2.0
