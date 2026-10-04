// The Claude Agent companion's entry point (ROADMAP §4.7; part
// `ClaudeAgentRuntime` in Agentique's model). The Studio starts it with Node
// (type stripping) for one turn and talks to it in protocol 2 on standard
// input and output. Two other modes serve Settings' health check:
//
//   main.ts --verify   the versions, and the Claude Code binary against the
//                      checksum in the SDK's own manifest
//   main.ts --probe    starts the SDK exactly as a turn does, with a
//                      placeholder key, and reports what the agent can do
//                      (its tools, MCP servers, permission mode) from the
//                      SDK's own start message; the refused key ends it
//                      before any model runs, so it costs nothing
//   main.ts --login    whether this computer has a Claude login (C-54), from
//                      the SDK's Claude Code binary's `claude auth status`
//                      with the Operator's own configuration: only whether
//                      and how, never a token, email or organisation;
//                      Agentique never uses it

import { query } from "@anthropic-ai/claude-agent-sdk/core";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { CallToolRequestSchema, ListToolsRequestSchema } from "@modelcontextprotocol/sdk/types.js";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream, readFileSync, statSync } from "node:fs";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { type McpFactory, Turn } from "./bridge.ts";
import { type CompanionMessage, LineSplitter, PROTOCOL, decode, encode } from "./protocol.ts";
import { SERVER, loginEnvironment, readLogin } from "./policy.ts";

const CLIENT = "agentique/1";

/** Agentique's tools as one MCP server, with their JSON Schemas unchanged. */
const mcpServer: McpFactory = (tools, call) => {
  const server = new McpServer({ name: SERVER, version: String(PROTOCOL) });
  server.server.registerCapabilities({ tools: { listChanged: false } });
  server.server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: tools.map((t) => ({
      name: t.name,
      description: t.description,
      inputSchema: t.inputSchema as { type: "object" },
      annotations: { readOnlyHint: t.readOnly === true },
      // Always in the prompt, never deferred behind tool search.
      _meta: { "anthropic/alwaysLoad": true },
    })),
  }));
  server.server.setRequestHandler(CallToolRequestSchema, async (request, extra) => {
    const answer = await call(
      request.params.name,
      (request.params.arguments ?? {}) as Record<string, unknown>,
      extra.signal,
    );
    return { content: [{ type: "text" as const, text: answer.content }], isError: answer.isError };
  });
  return server;
};

/** Where the SDK is installed, and what its manifest says. */
function sdkPackage(): { version: string; claudeCode: string; folder: string; manifest: Record<string, unknown> } {
  const folder = dirname(fileURLToPath(import.meta.resolve("@anthropic-ai/claude-agent-sdk/core")));
  const pkg = JSON.parse(readFileSync(join(folder, "package.json"), "utf8"));
  const manifest = JSON.parse(readFileSync(join(folder, "manifest.json"), "utf8"));
  return { version: String(pkg.version), claudeCode: String(manifest.version), folder, manifest };
}

function out(message: CompanionMessage): void {
  process.stdout.write(encode(message));
}

/** The SDK's Claude Code binary for this platform, as its manifest names it. */
function bundledBinary(): { binary: string; entry: Record<string, unknown> } | null {
  const sdk = sdkPackage();
  const platform = `${process.platform}-${process.arch}`;
  const entry = (sdk.manifest.platforms as Record<string, Record<string, unknown>>)?.[platform];
  if (!entry) {
    return null;
  }
  const binary = fileURLToPath(
    import.meta.resolve(`@anthropic-ai/claude-agent-sdk-${platform}/${String(entry.binary)}`),
  );
  return { binary, entry };
}

async function verify(): Promise<void> {
  const sdk = sdkPackage();
  const platform = `${process.platform}-${process.arch}`;
  const bundled = bundledBinary();
  const result: Record<string, unknown> = {
    protocol: PROTOCOL,
    node: process.version,
    sdk: sdk.version,
    claudeCode: sdk.claudeCode,
    platform,
  };
  if (!bundled) {
    result.binary = null;
    result.problem = `the SDK has no Claude Code binary for ${platform}`;
  } else {
    const { binary, entry } = bundled;
    const hash = createHash("sha256");
    await new Promise<void>((resolve, reject) => {
      createReadStream(binary).on("data", (d) => hash.update(d)).on("end", resolve).on("error", reject);
    });
    const digest = hash.digest("hex");
    result.binary = binary;
    result.size = statSync(binary).size;
    result.checksumMatches = digest === entry.checksum && result.size === entry.size;
  }
  process.stdout.write(`${JSON.stringify(result)}\n`);
}

/**
 * `claude auth status` with the Operator's own configuration (C-54): the
 * SDK's Claude Code binary, or a `claude` on the PATH without it. Prints
 * `{"login": {...}}` with only whether and how this computer is logged in,
 * or `{"login": null, "problem": "..."}`.
 */
async function login(): Promise<void> {
  let binary = "claude";
  try {
    binary = bundledBinary()?.binary ?? binary;
  } catch {
    // Not installed for this platform: a `claude` on the PATH, if any.
  }
  const result = await new Promise<{ stdout: string; error: string | null }>((resolve) => {
    execFile(
      binary,
      ["auth", "status", "--json"],
      { env: loginEnvironment(process.env), timeout: 20_000, windowsHide: true, maxBuffer: 1024 * 1024 },
      // A computer without a login exits non-zero with the same JSON.
      (error, stdout) => resolve({ stdout: String(stdout ?? ""), error: error && !stdout ? error.message : null }),
    );
  });
  const status = readLogin(result.stdout);
  process.stdout.write(
    `${JSON.stringify(
      status !== null
        ? { login: status }
        : { login: null, problem: result.error ?? "claude auth status did not answer in JSON" },
    )}\n`,
  );
}

async function probe(): Promise<void> {
  const base = await mkdtemp(join(tmpdir(), "agentique-probe-"));
  const from = {
    ...process.env,
    ANTHROPIC_API_KEY: "sk-ant-agentique-probe-not-a-key",
    CLAUDE_CODE_OAUTH_TOKEN: undefined,
  };
  const messages: CompanionMessage[] = [];
  const turn = new Turn((m) => messages.push(m), { query: query as never }, mcpServer, from, CLIENT);
  const timer = setTimeout(() => void turn.stop("the probe took too long"), 60_000);
  await turn.run({
    prompt: "Agentique's runtime probe.",
    systemPrompt: "A probe of Agentique's runtime configuration.",
    tools: [
      {
        name: "read_model",
        description: "Read the model (probe).",
        inputSchema: { type: "object", properties: {} },
        readOnly: true,
      },
    ],
    model: null,
    effort: null,
    resume: null,
    maxTurns: 1,
    cwd: base,
    configDir: join(base, "config"),
    home: base,
    policy: null,
    settingSources: [],
    agents: {},
    endpoint: null,
    preset: false,
  });
  clearTimeout(timer);
  const init = messages.find((m) => m.type === "init");
  const auth = messages.some((m) => m.type === "error" && m.kind === "auth");
  process.stdout.write(`${JSON.stringify({ init: init ?? null, keyRefusedAsExpected: auth })}\n`);
}

/** Protocol mode: one turn. */
async function serve(): Promise<void> {
  const sdk = sdkPackage();
  out({ type: "ready", protocol: PROTOCOL, sdk: sdk.version, claudeCode: sdk.claudeCode, node: process.version });
  let turn: Turn | null = null;
  let running: Promise<void> | null = null;
  const splitter = new LineSplitter();
  const fail = (message: string) => {
    out({ type: "error", kind: "protocol", message });
    void turn?.stop(message);
  };
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk: string) => {
    let lines: string[];
    try {
      lines = splitter.push(chunk);
    } catch (error) {
      fail(String(error));
      return;
    }
    for (const line of lines) {
      let message;
      try {
        message = decode(line);
      } catch (error) {
        fail(`not a protocol ${PROTOCOL} message: ${(error as Error).message}`);
        continue;
      }
      switch (message.type) {
        case "start":
          if (turn !== null) {
            fail("a turn is already running");
            break;
          }
          turn = new Turn(out, { query: query as never }, mcpServer, process.env, CLIENT);
          running = turn.run(message.options).then(() => process.exit(0));
          break;
        case "tool_result":
          turn?.answer(message.call, { content: message.content, isError: message.isError });
          break;
        case "permission_result":
          turn?.permit(message.call, message.allow, message.message);
          break;
        case "message":
          turn?.queue(message.text);
          break;
        case "gate":
          turn?.setGate(message.mode);
          break;
        case "interrupt":
          void turn?.stop("stopped by the Operator");
          break;
        case "close":
          void turn?.stop("the Studio closed the turn");
          if (running === null) {
            process.exit(0);
          }
          break;
      }
    }
  });
  // The Studio went away: nothing waits for it (fail closed).
  process.stdin.on("end", () => {
    void turn?.stop("the Studio is gone");
    if (running === null) {
      process.exit(0);
    }
    setTimeout(() => process.exit(1), 5_000);
  });
}

const mode = process.argv[2];
const work =
  mode === "--verify" ? verify() : mode === "--probe" ? probe() : mode === "--login" ? login() : serve();
work.catch((error) => {
  process.stderr.write(`${(error as Error).stack ?? String(error)}\n`);
  if (mode !== "--verify" && mode !== "--probe" && mode !== "--login") {
    out({ type: "error", kind: "runtime", message: String(error) });
  }
  process.exit(2);
});
