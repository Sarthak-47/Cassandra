import { installCassandraTransport } from "../dist/index.js";

const proxyUrl = process.env.CASSANDRA_OPENCODE_TRANSPORT_PROXY_URL;
if (!proxyUrl) {
  throw new Error("Cassandra OpenCode transport shim loaded without CASSANDRA_OPENCODE_TRANSPORT_PROXY_URL");
}

installCassandraTransport({ proxyUrl });
