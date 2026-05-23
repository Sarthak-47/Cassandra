import type { Plugin } from "@opencode-ai/plugin";
import { tool } from "@opencode-ai/plugin";
import { z } from "zod";

import { createCassandraRetrieveTool, getDefaultProxyUrl } from "./retrieve.js";
import { installCassandraTransport } from "./transport.js";

export interface CassandraOpenCodePluginOptions {
  proxyUrl?: string;
  project?: string;
  backend?: string;
  debug?: boolean;
}

function normalizeProxyUrl(url: string): string {
  return url.replace(/\/+$/, "");
}

function resolveProxyUrl(options?: CassandraOpenCodePluginOptions): string {
  return normalizeProxyUrl(
    options?.proxyUrl ??
      process.env.CASSANDRA_PROXY_URL ??
      process.env.CASSANDRA_BASE_URL ??
      getDefaultProxyUrl(),
  );
}

export const CassandraPlugin: Plugin = async (input, options = {}) => {
  const pluginOptions = options as CassandraOpenCodePluginOptions;
  const proxyUrl = resolveProxyUrl(pluginOptions);
  const retrieveTool = createCassandraRetrieveTool({ proxyBaseUrl: proxyUrl });
  const uninstallTransport = installCassandraTransport({
    proxyUrl,
    debug: pluginOptions.debug,
  });

  return {
    dispose: async () => {
      uninstallTransport();
    },
    tool: {
      cassandra_retrieve: tool({
        description: retrieveTool.description,
        args: {
          hash: z
            .string()
            .regex(/^[a-f0-9]{24}$/i, "Expected 24-character hex hash"),
        },
        async execute(args) {
          return retrieveTool.execute(args);
        },
      }),
    },
    "shell.env": async (_input, output) => {
      output.env.CASSANDRA_ACTIVE = "1";
      output.env.CASSANDRA_PROXY_URL = proxyUrl;
      output.env.CASSANDRA_PROJECT =
        pluginOptions.project ??
        (input.project as { id?: string }).id ??
        input.directory;
      if (pluginOptions.backend) {
        output.env.CASSANDRA_BACKEND = pluginOptions.backend;
      }
    },
  };
};

export default CassandraPlugin;
