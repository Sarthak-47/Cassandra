import { afterEach, describe, expect, it, vi } from "vitest";

const mocked = vi.hoisted(() => ({
  ensureProxyUrl: vi.fn(async () => "http://127.0.0.1:8787"),
  ensureProxyStarted: vi.fn(),
  getProxyUrl: vi.fn(() => null as string | null),
  createCassandraRetrieveTool: vi.fn(({ proxyUrl }: { proxyUrl: string }) => ({ proxyUrl })),
}));

const proxyReadyListeners: Array<(proxyUrl: string) => void | Promise<void>> = [];

vi.mock("../src/engine.js", () => ({
  CassandraContextEngine: class {
    ensureProxyUrl = mocked.ensureProxyUrl;
    ensureProxyStarted = mocked.ensureProxyStarted;
    getProxyUrl = mocked.getProxyUrl;
    onProxyReady(listener: (proxyUrl: string) => void | Promise<void>) {
      proxyReadyListeners.push(listener);
      return () => {};
    }
  },
}));

vi.mock("../src/tools/cassandra-retrieve.js", () => ({
  createCassandraRetrieveTool: mocked.createCassandraRetrieveTool,
}));

import cassandraPlugin from "../src/plugin/index.js";

afterEach(() => {
  vi.restoreAllMocks();
  mocked.ensureProxyUrl.mockClear();
  mocked.ensureProxyStarted.mockClear();
  mocked.getProxyUrl.mockReset();
  mocked.getProxyUrl.mockReturnValue(null);
  mocked.createCassandraRetrieveTool.mockClear();
  proxyReadyListeners.length = 0;
});

describe("cassandraPlugin runtime routing", () => {
  function stubConfiguredProxyProbe(response: "cassandra" | "non-cassandra" | "down") {
    if (response === "down") {
      vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("ECONNREFUSED")));
      return;
    }

    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (url.endsWith("/readyz")) {
          return Promise.resolve({ ok: true, status: 200, text: () => Promise.resolve("") });
        }
        if (url.endsWith("/v1/retrieve/stats")) {
          return Promise.resolve({
            ok: response === "cassandra",
            status: response === "cassandra" ? 200 : 404,
            text: () => Promise.resolve(""),
          });
        }
        if (url.endsWith("/stats")) {
          return Promise.resolve({
            ok: response === "cassandra",
            status: response === "cassandra" ? 200 : 200,
            text: () =>
              Promise.resolve(response === "cassandra" ? JSON.stringify({ proxy_inbound: { total: 1 } }) : "{}"),
          });
        }
        return Promise.resolve({ ok: false, status: 404, text: () => Promise.resolve("") });
      }),
    );
  }

  it("routes configured providers in memory once the proxy becomes available", async () => {
    const gatewayHandlers = new Map<string, () => Promise<void>>();
    const writeConfigFile = vi.fn();
    const loadConfig = vi.fn(() => ({
      models: {
        providers: {
          anthropic: {
            api: "anthropic-messages",
          },
        },
      },
    }));

    const api: any = {
      config: {
        plugins: {
          entries: {
            cassandra: {
              config: {
                gatewayProviderIds: ["codex", "claude", "copilot", "gemini", "openrouter"],
              },
            },
          },
        },
        models: {
          providers: {
            anthropic: {
              api: "anthropic-messages",
              baseUrl: "https://api.anthropic.com",
            },
            "github-copilot": {
              baseUrl: "https://api.githubcopilot.com/v1",
            },
            google: {
              baseUrl: "https://generativelanguage.googleapis.com/v1beta",
            },
            openrouter: {
              baseUrl: "https://openrouter.ai/api/v1",
            },
          },
        },
      },
      logger: {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
      },
      registerContextEngine: vi.fn(),
      registerTool: vi.fn(),
      on: vi.fn((event: string, handler: () => Promise<void>) => {
        gatewayHandlers.set(event, handler);
      }),
      runtime: {
        config: {
          loadConfig,
          writeConfigFile,
        },
      },
    };

    cassandraPlugin(api);
    await Promise.resolve();

    // With no active or configured proxy URL, initial routing defers without
    // auto-starting the proxy or mutating providers.
    expect(mocked.ensureProxyUrl).not.toHaveBeenCalled();
    expect(mocked.ensureProxyStarted).not.toHaveBeenCalled();
    expect(writeConfigFile).not.toHaveBeenCalled();
    expect(loadConfig).not.toHaveBeenCalled();
    expect(api.config.models.providers["openai-codex"]).toBeUndefined();

    await proxyReadyListeners[0]?.("http://127.0.0.1:8787");

    expect(api.config.models.providers["openai-codex"]).toEqual({
      baseUrl: "http://127.0.0.1:8787/backend-api",
      models: [],
    });
    expect(api.config.models.providers.anthropic).toEqual({
      api: "anthropic-messages",
      baseUrl: "http://127.0.0.1:8787",
      models: [],
    });
    expect(api.config.models.providers["github-copilot"]).toEqual({
      baseUrl: "http://127.0.0.1:8787/v1",
      models: [],
    });
    expect(api.config.models.providers.google).toEqual({
      baseUrl: "http://127.0.0.1:8787/v1beta",
      models: [],
    });
    expect(api.config.models.providers.openrouter).toEqual({
      baseUrl: "http://127.0.0.1:8787/api/v1",
      models: [],
    });

    const gatewayStart = gatewayHandlers.get("gateway_start");
    expect(gatewayStart).toBeTypeOf("function");
    // getProxyUrl now reports the active proxy so gateway_start re-routes in
    // memory without ever auto-starting or awaiting the proxy.
    mocked.getProxyUrl.mockReturnValue("http://127.0.0.1:8787");
    await gatewayStart?.();
    expect(mocked.ensureProxyStarted).not.toHaveBeenCalled();
    expect(mocked.ensureProxyUrl).not.toHaveBeenCalled();
    expect(writeConfigFile).not.toHaveBeenCalled();
    expect(loadConfig).not.toHaveBeenCalled();
  });

  it("does not auto-start on gateway_start when no proxy URL is available", async () => {
    const gatewayHandlers = new Map<string, () => Promise<void>>();

    const api: any = {
      config: {
        plugins: {
          entries: {
            cassandra: {
              config: { gatewayProviderIds: ["claude"] },
            },
          },
        },
        models: {
          providers: {
            anthropic: { api: "anthropic-messages", baseUrl: "https://api.anthropic.com" },
          },
        },
      },
      logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() },
      registerContextEngine: vi.fn(),
      registerTool: vi.fn(),
      on: vi.fn((event: string, handler: () => Promise<void>) => {
        gatewayHandlers.set(event, handler);
      }),
    };

    cassandraPlugin(api);
    await Promise.resolve();

    await gatewayHandlers.get("gateway_start")?.();

    expect(mocked.ensureProxyStarted).not.toHaveBeenCalled();
    expect(mocked.ensureProxyUrl).not.toHaveBeenCalled();
    expect(api.config.models.providers.anthropic).toEqual({
      api: "anthropic-messages",
      baseUrl: "https://api.anthropic.com",
    });
  });

  it("routes configured proxyUrl only after it probes as Cassandra", async () => {
    const gatewayHandlers = new Map<string, () => Promise<void>>();
    stubConfiguredProxyProbe("cassandra");

    const api: any = {
      config: {
        plugins: {
          entries: {
            cassandra: {
              config: {
                proxyUrl: "http://127.0.0.1:8787",
                gatewayProviderIds: ["claude"],
              },
            },
          },
        },
        models: {
          providers: {
            anthropic: { api: "anthropic-messages", baseUrl: "https://api.anthropic.com" },
          },
        },
      },
      logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() },
      registerContextEngine: vi.fn(),
      registerTool: vi.fn(),
      on: vi.fn((event: string, handler: () => Promise<void>) => {
        gatewayHandlers.set(event, handler);
      }),
    };

    cassandraPlugin(api);
    await gatewayHandlers.get("gateway_start")?.();

    // Configured proxyUrl is probe-gated before provider mutation.
    expect(mocked.ensureProxyStarted).not.toHaveBeenCalled();
    expect(mocked.ensureProxyUrl).not.toHaveBeenCalled();
    expect(api.config.models.providers.anthropic).toEqual({
      api: "anthropic-messages",
      baseUrl: "http://127.0.0.1:8787",
      models: [],
    });
  });

  it("does not route configured proxyUrl when the proxy is unavailable", async () => {
    const gatewayHandlers = new Map<string, () => Promise<void>>();
    stubConfiguredProxyProbe("down");

    const api: any = {
      config: {
        plugins: {
          entries: {
            cassandra: {
              config: {
                proxyUrl: "http://127.0.0.1:8787",
                gatewayProviderIds: ["claude"],
              },
            },
          },
        },
        models: {
          providers: {
            anthropic: { api: "anthropic-messages", baseUrl: "https://api.anthropic.com" },
          },
        },
      },
      logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() },
      registerContextEngine: vi.fn(),
      registerTool: vi.fn(),
      on: vi.fn((event: string, handler: () => Promise<void>) => {
        gatewayHandlers.set(event, handler);
      }),
    };

    cassandraPlugin(api);
    await Promise.resolve();
    await Promise.resolve();
    await gatewayHandlers.get("gateway_start")?.();

    expect(mocked.ensureProxyStarted).not.toHaveBeenCalled();
    expect(mocked.ensureProxyUrl).not.toHaveBeenCalled();
    expect(api.config.models.providers.anthropic).toEqual({
      api: "anthropic-messages",
      baseUrl: "https://api.anthropic.com",
    });
    expect(api.logger.warn).toHaveBeenCalledWith(
      expect.stringContaining("Skipping upstream gateway routing"),
    );
  });

  it("does not route configured proxyUrl when only generic liveness endpoints respond", async () => {
    const gatewayHandlers = new Map<string, () => Promise<void>>();
    stubConfiguredProxyProbe("non-cassandra");

    const api: any = {
      config: {
        plugins: {
          entries: {
            cassandra: {
              config: {
                proxyUrl: "http://127.0.0.1:8787",
                gatewayProviderIds: ["claude"],
              },
            },
          },
        },
        models: {
          providers: {
            anthropic: { api: "anthropic-messages", baseUrl: "https://api.anthropic.com" },
          },
        },
      },
      logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() },
      registerContextEngine: vi.fn(),
      registerTool: vi.fn(),
      on: vi.fn((event: string, handler: () => Promise<void>) => {
        gatewayHandlers.set(event, handler);
      }),
    };

    cassandraPlugin(api);
    await gatewayHandlers.get("gateway_start")?.();

    expect(api.config.models.providers.anthropic).toEqual({
      api: "anthropic-messages",
      baseUrl: "https://api.anthropic.com",
    });
    expect(api.logger.warn).toHaveBeenCalledWith(
      expect.stringContaining("configured proxyUrl is not a ready Cassandra proxy"),
    );
  });

  it("documents that the retrieve tool can be created from configured proxyUrl before routing is validated", () => {
    stubConfiguredProxyProbe("down");

    const api: any = {
      config: {
        plugins: {
          entries: {
            cassandra: {
              config: {
                proxyUrl: "http://127.0.0.1:8787",
                gatewayProviderIds: ["codex"],
              },
            },
          },
        },
        models: {
          providers: {},
        },
      },
      logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() },
      registerContextEngine: vi.fn(),
      registerTool: vi.fn(),
      on: vi.fn(),
    };

    cassandraPlugin(api);
    const [toolFactory] = api.registerTool.mock.calls[0];
    const tool = toolFactory({});

    expect(tool).toEqual({ proxyUrl: "http://127.0.0.1:8787" });
    expect(mocked.createCassandraRetrieveTool).toHaveBeenCalledWith({
      proxyUrl: "http://127.0.0.1:8787",
    });
  });
});
