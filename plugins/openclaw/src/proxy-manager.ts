/**
 * Manages connectivity to a Cassandra proxy (local or remote).
 *
 * Security model:
 * - Local proxies (127.0.0.1 / localhost) can be auto-started via subprocess
 * - Remote proxies are connect-only: probe and use, never launch
 * - No environment variable access
 */
import { spawn } from "node:child_process";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export interface ProxyManagerConfig {
  proxyUrl?: string;
  proxyPort?: number;
  pythonPath?: string;
  autoStart?: boolean;
  startupTimeoutMs?: number;
  retryMaxAttempts?: number;
  connectTimeoutSeconds?: number;
}

export interface ProxyManagerLogger {
  info(message: string): void;
  warn(message: string): void;
  error(message: string): void;
  debug(message: string): void;
}

/** Default logger that prefixes all messages with `[cassandra]`. */
export const defaultLogger: ProxyManagerLogger = {
  info: (m) => console.log(`[cassandra] ${m}`),
  warn: (m) => console.warn(`[cassandra] ${m}`),
  error: (m) => console.error(`[cassandra] ${m}`),
  debug: () => {},
};

export interface ProxyProbeResult {
  reachable: boolean;
  isCassandra: boolean;
  reason?: string;
}

interface LaunchSpec {
  label: string;
  command: string;
  args: string[];
  checkCommand: string;
  checkArgs: string[];
  useShell?: boolean;
  checkUseShell?: boolean;
}

const CASSANDRA_MODULE_DISCOVERY_SNIPPET =
  "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec('cassandra') else 1)";

export class ProxyManager {
  private config: ProxyManagerConfig;
  private logger: ProxyManagerLogger;
  private proxyUrl: string | null = null;

  constructor(config: ProxyManagerConfig = {}, logger?: ProxyManagerLogger) {
    this.config = config;
    this.logger = logger ?? defaultLogger;
  }

  /**
   * Ensure a proxy is available. Returns the normalized URL origin.
   */
  async start(): Promise<string> {
    const port = this.getProxyPort();
    const rawExplicitUrl =
      typeof this.config.proxyUrl === "string" && this.config.proxyUrl.trim().length > 0
        ? normalizeAndValidateProxyUrl(this.config.proxyUrl)
        : null;
    // Only apply proxyPort default to local URLs — remote URLs use their protocol default
    const explicitUrl = rawExplicitUrl
      ? isLocalProxyUrl(rawExplicitUrl) ? withDefaultPort(rawExplicitUrl, port) : rawExplicitUrl
      : null;
    const defaultCandidates = this.getDefaultProxyCandidates(port);
    const candidateUrls = explicitUrl ? [explicitUrl] : [...defaultCandidates];
    const probeByUrl = new Map<string, ProxyProbeResult>();

    for (const url of candidateUrls) {
      const probe = await probeCassandraProxy(url);
      probeByUrl.set(url, probe);
      if (probe.reachable && probe.isCassandra) {
        this.proxyUrl = url;
        this.logger.info(`Cassandra proxy already running at ${url}`);
        return url;
      }
    }

    if (explicitUrl) {
      const explicitProbe = probeByUrl.get(explicitUrl);
      if (explicitProbe?.reachable && !explicitProbe.isCassandra) {
        throw new Error(
          `Service reachable at ${explicitUrl}, but it does not appear to be a Cassandra proxy (${explicitProbe.reason ?? "unknown service"}).`,
        );
      }
    }

    // Remote URLs are connect-only — never auto-start a subprocess for them
    if (explicitUrl && !isLocalProxyUrl(explicitUrl)) {
      throw new Error(
        `Remote Cassandra proxy not reachable at ${explicitUrl}. Ensure the proxy is running at that address.`,
      );
    }

    // Auto-start is only available for local proxies
    if (this.config.autoStart === true) {
      const startupUrl = explicitUrl ?? defaultCandidates[0];
      const startupProbe = probeByUrl.get(startupUrl);
      if (startupProbe?.reachable && !startupProbe.isCassandra) {
        throw new Error(
          `Cannot auto-start Cassandra at ${startupUrl}: port is in use by a non-Cassandra service (${startupProbe.reason ?? "unknown service"}).`,
        );
      }

      this.logger.info(
        `No Cassandra proxy detected${explicitUrl ? ` at ${startupUrl}` : " on default local endpoints"}; attempting to auto-start...`,
      );
      await this.startCassandraProxy(startupUrl, port);

      const startedProbe = await waitForCassandraProxy(
        startupUrl,
        this.config.startupTimeoutMs ?? 20_000,
      );
      if (startedProbe.reachable && startedProbe.isCassandra) {
        this.proxyUrl = startupUrl;
        this.logger.info(`Cassandra proxy started and reachable at ${startupUrl}`);
        return startupUrl;
      }
      throw new Error(
        `Attempted to start Cassandra proxy, but it was not reachable at ${startupUrl} (${startedProbe.reason ?? "unknown"}).`,
      );
    }

    if (explicitUrl) {
      throw new Error(
        `Cassandra proxy not reachable at ${explicitUrl}. Ensure the proxy is running first.`,
      );
    }

    throw new Error(
      `Cassandra proxy not detected on default endpoints (${defaultCandidates.join(", ")}). ` +
        "Set proxyUrl explicitly or enable autoStart.",
    );
  }

  private getProxyPort(): number {
    const rawPort = this.config.proxyPort;
    if (!Number.isInteger(rawPort) || rawPort === undefined) return 8787;
    if (rawPort < 1 || rawPort > 65535) {
      throw new Error("proxyPort must be an integer between 1 and 65535");
    }
    return rawPort;
  }

  private getDefaultProxyCandidates(port: number): string[] {
    return [`http://127.0.0.1:${port}`, `http://localhost:${port}`];
  }

  /**
   * Stop manager state. Spawned proxy processes are detached and externally managed.
   */
  async stop(): Promise<void> {
    this.proxyUrl = null;
  }

  getUrl(): string | null {
    return this.proxyUrl;
  }

  // --- Internal ---

  private async startCassandraProxy(proxyUrl: string, defaultPort: number): Promise<void> {
    const parsed = new URL(proxyUrl);
    const host = parsed.hostname;
    const port = parsed.port || String(defaultPort);
    const specs = this.buildLaunchSpecs(host, port);
    const errors: string[] = [];

    for (const spec of specs) {
      if (!this.canExecute(spec.checkCommand, spec.checkArgs, spec.checkUseShell ?? spec.useShell)) {
        this.logger.debug(`Launcher unavailable: ${spec.label}`);
        continue;
      }

      try {
        const child = spawn(spec.command, spec.args, {
          detached: true,
          shell: spec.useShell === true,
          stdio: "ignore",
        });
        child.unref();
        this.logger.info(`Auto-start launcher selected: ${spec.label}`);
        return;
      } catch (error) {
        errors.push(`${spec.label}: ${String(error)}`);
      }
    }

    throw new Error(
      "No usable Cassandra launcher found. Tried PATH, local npm, global npm, and Python. " +
        "Install cassandra-ai (npm or pip) and ensure one launcher is available.\n" +
        (errors.length > 0 ? `Launch errors: ${errors.join("; ")}` : ""),
    );
  }

  private buildLaunchSpecs(host: string, port: string): LaunchSpec[] {
    const commonArgs = ["proxy", "--host", host, "--port", port];
    const retryMaxAttempts = this.config.retryMaxAttempts;
    if (Number.isInteger(retryMaxAttempts)) {
      commonArgs.push("--retry-max-attempts", String(retryMaxAttempts));
    }

    const connectTimeoutSeconds = this.config.connectTimeoutSeconds;
    if (Number.isInteger(connectTimeoutSeconds)) {
      commonArgs.push("--connect-timeout-seconds", String(connectTimeoutSeconds));
    }

    const specs: LaunchSpec[] = [];

    const configuredPython = this.getConfiguredPythonCommand();
    if (configuredPython) {
      specs.push({
        label: `Configured Python: ${configuredPython} -m cassandra.cli`,
        command: configuredPython,
        args: ["-m", "cassandra.cli", ...commonArgs],
        checkCommand: configuredPython,
        checkArgs: ["-c", CASSANDRA_MODULE_DISCOVERY_SNIPPET],
      });
    }

    // 2) Windows pyenv: resolve the real executable so we avoid shim .bat wrappers.
    if (process.platform === "win32") {
      const pyenvCassandra = this.getPyenvResolvedCassandra();
      if (pyenvCassandra) {
        specs.push({
          label: `pyenv: ${pyenvCassandra}`,
          command: pyenvCassandra,
          args: commonArgs,
          checkCommand: pyenvCassandra,
          checkArgs: ["--version"],
          useShell: false,
        });
      }
    }

    // 3) PATH
    specs.push({
      label: "PATH: cassandra",
      command: "cassandra",
      args: commonArgs,
      checkCommand: process.platform === "win32" ? "where.exe" : "sh",
      checkArgs: process.platform === "win32"
        ? ["cassandra"]
        : ["-lc", "command -v cassandra >/dev/null 2>&1"],
      useShell: process.platform === "win32",
      checkUseShell: false,
    });

    // 4) Local npm install (inside plugin install path)
    const moduleDir = dirname(fileURLToPath(import.meta.url)); // .../dist
    const packageRoot = dirname(moduleDir);
    const localBinDir = join(packageRoot, "node_modules", ".bin");
    const localBins = process.platform === "win32"
      ? [join(localBinDir, "cassandra.cmd"), join(localBinDir, "cassandra")]
      : [join(localBinDir, "cassandra")];
    for (const localBin of localBins) {
      if (!existsSync(localBin)) continue;
        specs.push({
          label: `Local npm: ${localBin}`,
          command: localBin,
          args: commonArgs,
          checkCommand: localBin,
          checkArgs: ["--version"],
          useShell: process.platform === "win32",
        });
      }

    // 5) Global npm install
    const npmPrefix = this.getNpmGlobalPrefix();
    if (npmPrefix) {
      const globalBins = process.platform === "win32"
        ? [join(npmPrefix, "cassandra.cmd"), join(npmPrefix, "cassandra")]
        : [join(npmPrefix, "bin", "cassandra"), join(npmPrefix, "cassandra")];

      for (const globalBin of globalBins) {
        if (!existsSync(globalBin)) continue;
        specs.push({
          label: `Global npm: ${globalBin}`,
          command: globalBin,
          args: commonArgs,
          checkCommand: globalBin,
          checkArgs: ["--version"],
          useShell: process.platform === "win32",
        });
      }
    }

    // 6) Python module fallback
    const pythonCommands = this.getPythonCommands();
    for (const pyCmd of pythonCommands) {
      if (configuredPython && pyCmd === configuredPython) continue;
      specs.push({
        label: `Python: ${pyCmd} -m cassandra.cli`,
        command: pyCmd,
        args: ["-m", "cassandra.cli", ...commonArgs],
        checkCommand: pyCmd,
        checkArgs: ["-c", CASSANDRA_MODULE_DISCOVERY_SNIPPET],
      });
    }

    return specs;
  }

  private getConfiguredPythonCommand(): string | null {
    const configured = typeof this.config.pythonPath === "string"
      ? this.config.pythonPath.trim()
      : "";
    return configured.length > 0 ? configured : null;
  }

  private getPyenvResolvedCassandra(): string | null {
    if (process.platform !== "win32") return null;

    try {
      const result = spawnSync("pyenv", ["which", "cassandra"], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
        timeout: 5000,
      });
      if (result.error || result.status !== 0) return null;
      const resolved = (result.stdout ?? "").trim().split(/\r?\n/, 1)[0];
      if (!resolved || !existsSync(resolved)) return null;
      return resolved;
    } catch {
      return null;
    }
  }

  private getPythonCommands(): string[] {
    const commands: string[] = [];
    const configured = this.getConfiguredPythonCommand() ?? "";
    if (configured.length > 0) {
      commands.push(configured);
    }
    for (const fallback of ["python", "python3", "py"]) {
      if (!commands.includes(fallback)) commands.push(fallback);
    }
    return commands;
  }

  private canExecute(command: string, args: string[], useShell = false): boolean {
    try {
      const result = spawnSync(command, args, {
        shell: useShell,
        stdio: "ignore",
        timeout: 5000,
      });
      if (result.error) return false;
      return result.status === 0;
    } catch {
      return false;
    }
  }

  private getNpmGlobalPrefix(): string | null {
    try {
      const result = spawnSync("npm", ["prefix", "-g"], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
        timeout: 5000,
      });
      if (result.error || result.status !== 0) return null;
      const prefix = (result.stdout ?? "").trim();
      return prefix.length > 0 ? prefix : null;
    } catch {
      return null;
    }
  }
}

/** Parse a URL, returning the parsed object or throwing a descriptive error. */
function parseProxyUrl(proxyUrl: string): URL {
  try {
    return new URL(proxyUrl);
  } catch {
    throw new Error(`Invalid proxyUrl: "${proxyUrl}"`);
  }
}

export function normalizeAndValidateProxyUrl(proxyUrl: string): string {
  const parsed = parseProxyUrl(proxyUrl);

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("proxyUrl must use http:// or https://");
  }

  if (parsed.pathname !== "/" || parsed.search || parsed.hash) {
    throw new Error("proxyUrl must not include a path, query, or hash");
  }

  return parsed.origin;
}

/** Returns true if the URL points to a local address (localhost or 127.0.0.1). */
export function isLocalProxyUrl(proxyUrl: string): boolean {
  try {
    const parsed = new URL(proxyUrl);
    return parsed.hostname === "127.0.0.1" || parsed.hostname === "localhost";
  } catch {
    return false;
  }
}

function withDefaultPort(proxyUrl: string, defaultPort: number): string {
  const parsed = parseProxyUrl(proxyUrl);
  if (!parsed.port) {
    parsed.port = String(defaultPort);
  }
  return parsed.origin;
}

/**
 * Probe a configured URL and verify whether it is a running Cassandra proxy.
 */
export async function probeCassandraProxy(proxyUrl: string): Promise<ProxyProbeResult> {
  const origin = normalizeAndValidateProxyUrl(proxyUrl);
  const probeEndpoint = async (
    path: string,
    options: { readBody?: boolean } = {},
  ): Promise<{ reachable: boolean; ok: boolean; status?: number; body?: string }> => {
    try {
      const response = await fetch(`${origin}${path}`, {
        signal: AbortSignal.timeout(3_000),
      });
      const body =
        response.ok && options.readBody
          ? await response.text().catch(() => undefined)
          : undefined;
      return { reachable: true, ok: response.ok, status: response.status, body };
    } catch {
      return { reachable: false, ok: false };
    }
  };

  const ready = await probeEndpoint("/readyz");
  const retrieveStats = await probeEndpoint("/v1/retrieve/stats", { readBody: true });
  if (retrieveStats.ok && hasCassandraStatsShape(retrieveStats.body)) {
    return { reachable: true, isCassandra: true };
  }

  const stats = await probeEndpoint("/stats", { readBody: true });
  if (stats.ok && hasCassandraStatsShape(stats.body)) {
    return { reachable: true, isCassandra: true };
  }

  const health = await probeEndpoint("/health");
  const anyReachable = ready.reachable || retrieveStats.reachable || stats.reachable || health.reachable;
  if (!anyReachable) {
    return { reachable: false, isCassandra: false, reason: "proxy probe failed" };
  }

  const reasons = [
    ready.reachable ? `readyz HTTP ${ready.status}` : "readyz unavailable",
    retrieveStats.reachable
      ? `retrieve stats HTTP ${retrieveStats.status}`
      : "retrieve stats endpoint unavailable",
    stats.reachable ? `stats HTTP ${stats.status}` : "stats endpoint unavailable",
    health.reachable ? `health HTTP ${health.status}` : "health check failed",
  ];
  return { reachable: true, isCassandra: false, reason: reasons.join("; ") };
}

function hasCassandraStatsShape(body: string | undefined): boolean {
  if (!body) {
    return false;
  }

  try {
    const parsed = JSON.parse(body) as Record<string, unknown>;
    return (
      parsed !== null &&
      typeof parsed === "object" &&
      (Object.hasOwn(parsed, "proxy_inbound") ||
        Object.hasOwn(parsed, "api_requests") ||
        Object.hasOwn(parsed, "provider_tokens") ||
        Object.hasOwn(parsed, "proxy_compression_saved") ||
        Object.hasOwn(parsed, "store") ||
        Object.hasOwn(parsed, "recent_retrievals"))
    );
  } catch {
    return false;
  }
}

async function waitForCassandraProxy(proxyUrl: string, timeoutMs: number): Promise<ProxyProbeResult> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const result = await probeCassandraProxy(proxyUrl);
    if (result.reachable && result.isCassandra) {
      return result;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  return probeCassandraProxy(proxyUrl);
}
