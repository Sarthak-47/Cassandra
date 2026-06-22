# TypeScript SDK

The Cassandra TypeScript SDK lets any JavaScript or TypeScript application compress LLM messages before sending them to a model. It saves tokens, reduces costs, and fits more context into every request.

## Install

```bash
npm install cassandra-ai
```

Requires a running [Cassandra proxy](proxy.md).

## Quick Start

```typescript
import { compress } from 'cassandra-ai';

const result = await compress(messages, { model: 'gpt-4o' });
console.log(`Saved ${result.tokensSaved} tokens`);

const response = await openai.chat.completions.create({
  model: 'gpt-4o',
  messages: result.messages,
});
```

## How It Works

The TypeScript SDK is an HTTP client. When you call `compress()`, it sends your messages to the Cassandra proxy's `POST /v1/compress` endpoint. The proxy runs the full compression pipeline (SmartCrusher, ContentRouter, CacheAligner, etc.) and returns compressed messages. No compression logic runs in Node.js — all the heavy lifting happens in the proxy.

```
Your TypeScript App
    │
    │  compress(messages)
    ▼
cassandra-ai (npm)  ← HTTP client
    │
    │  POST /v1/compress
    ▼
Cassandra Proxy / Cloud  ← compression pipeline (Python)
    │
    │  compressed messages
    ▼
Your TypeScript App
    │
    │  openai.chat.completions.create(compressed)
    ▼
LLM Provider
```

## Core API: `compress()`

```typescript
import { compress } from 'cassandra-ai';

const result = await compress(messages, {
  model: 'gpt-4o',                      // model name (for token counting)
  baseUrl: 'http://localhost:8787',      // proxy URL (default)
  apiKey: 'your-api-key',                // optional, for authenticated endpoints
  timeout: 30000,                        // ms (default)
  fallback: true,                        // return uncompressed if proxy down (default)
  retries: 1,                            // retry on transient errors (default)
});

result.messages          // compressed messages (same format as input)
result.tokensBefore      // original token count
result.tokensAfter       // compressed token count
result.tokensSaved       // tokens removed
result.compressionRatio  // tokensAfter / tokensBefore
result.transformsApplied // e.g. ['router:smart_crusher:0.35']
result.compressed        // false if fallback kicked in
```

Messages use standard OpenAI chat format: `{ role, content, tool_calls?, tool_call_id? }`.

### Environment Variables

Instead of passing options, set environment variables:

- `CASSANDRA_BASE_URL` — proxy URL (default: `http://localhost:8787`)
- `CASSANDRA_API_KEY` — optional API key for authenticated endpoints

## Reusable Client

For apps making many calls, create a client once and reuse it:

```typescript
import { CassandraClient } from 'cassandra-ai';

const client = new CassandraClient({
  baseUrl: 'http://localhost:8787',
  apiKey: 'your-api-key',
});

const r1 = await client.compress(messages1, { model: 'gpt-4o' });
const r2 = await client.compress(messages2, { model: 'gpt-4o' });
```

## Framework Adapters

### Vercel AI SDK

The Cassandra middleware plugs directly into Vercel AI SDK's `wrapLanguageModel()`:

```typescript
import { cassandraMiddleware } from 'cassandra-ai/vercel-ai';
import { wrapLanguageModel, generateText } from 'ai';
import { openai } from '@ai-sdk/openai';

const model = wrapLanguageModel({
  model: openai('gpt-4o'),
  middleware: cassandraMiddleware(),
});

// All calls through this model are automatically compressed
const { text } = await generateText({ model, messages });
```

The middleware intercepts messages in the `transformParams` hook, converts Vercel's internal format to OpenAI format, compresses via the proxy, and converts back. Your app code doesn't change.

You can also compress Vercel messages directly:

```typescript
import { compressVercelMessages } from 'cassandra-ai/vercel-ai';

const result = await compressVercelMessages(modelMessages, { model: 'gpt-4o' });
// result.messages is in Vercel ModelMessage[] format
```

### OpenAI SDK

Wrap your OpenAI client to auto-compress messages on every `chat.completions.create()` call:

```typescript
import { withCassandra } from 'cassandra-ai/openai';
import OpenAI from 'openai';

const client = withCassandra(new OpenAI());

// Messages are compressed before sending — transparent to your code
const response = await client.chat.completions.create({
  model: 'gpt-4o',
  messages: longConversation,
});
```

Only `chat.completions.create()` is intercepted. All other methods (embeddings, images, audio) pass through unchanged.

### Anthropic SDK

Same pattern for the Anthropic client:

```typescript
import { withCassandra } from 'cassandra-ai/anthropic';
import Anthropic from '@anthropic-ai/sdk';

const client = withCassandra(new Anthropic());

const response = await client.messages.create({
  model: 'claude-sonnet-4-5-20250929',
  messages: longConversation,
  max_tokens: 1024,
});
```

Only `messages.create()` is intercepted. The adapter converts between Anthropic's content block format and OpenAI format automatically.

## Error Handling

```typescript
import { compress, CassandraConnectionError, CassandraAuthError } from 'cassandra-ai';

try {
  const result = await compress(messages, { model: 'gpt-4o', fallback: false });
} catch (error) {
  if (error instanceof CassandraAuthError) {
    // Invalid API key (401)
  } else if (error instanceof CassandraConnectionError) {
    // Proxy unreachable
  }
}
```

With `fallback: true` (the default), connection errors and 5xx responses return the original messages uncompressed instead of throwing. Auth errors (401) and client errors (400) always throw.

## Fallback Behavior

By default, `compress()` never blocks your app. If the proxy is unreachable:

| Scenario | `fallback: true` (default) | `fallback: false` |
|----------|---------------------------|-------------------|
| Proxy unreachable | Returns uncompressed, `compressed: false` | Throws `CassandraConnectionError` |
| Proxy 503 error | Returns uncompressed after retries | Throws `CassandraCompressError` |
| Invalid API key (401) | Throws `CassandraAuthError` | Throws `CassandraAuthError` |
| Bad request (400) | Throws `CassandraCompressError` | Throws `CassandraCompressError` |

## Zero Dependencies

The `cassandra-ai` package has no runtime dependencies. Framework SDKs (Vercel AI, OpenAI, Anthropic) are optional peer dependencies — only install what you use.

## OpenClaw Plugin

The TypeScript SDK powers the [`cassandra-openclaw`](https://www.npmjs.com/package/cassandra-openclaw) plugin for [OpenClaw](https://github.com/openclaw/openclaw) agents. The plugin uses `CassandraClient` internally to compress context during the `assemble()` lifecycle hook. The preferred install flow is `cassandra wrap openclaw`; the direct plugin command is `openclaw plugins install --dangerously-force-unsafe-install cassandra-ai/openclaw`. See the [plugin source](https://github.com/Sarthak-47/cassandra/tree/main/plugins/openclaw) for details.

## Comparison with Python SDK

| Feature | Python SDK | TypeScript SDK |
|---------|-----------|---------------|
| `compress()` | Native (runs locally) | HTTP client (calls proxy) |
| Proxy | Built-in server | Connects to proxy |
| Vercel AI SDK | N/A | Middleware adapter |
| OpenAI SDK | `CassandraClient` wrapper | `withCassandra()` wrapper |
| Anthropic SDK | `CassandraClient` wrapper | `withCassandra()` wrapper |
| LangChain | `CassandraChatModel` | Use `compress()` directly |
| Memory system | Full (SQLite + HNSW) | Not yet (use proxy) |
| MCP server | Built-in | Not yet |
| CLI tools | `cassandra proxy`, `cassandra wrap`, etc. | N/A (use Python CLI) |
