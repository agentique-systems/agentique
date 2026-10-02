// One turn of the Claude Agent runtime (ROADMAP §4.7, §4.10): the SDK runs
// the loop; this file only carries what happens to the Studio and the
// Studio's answers back. Tool calls go to the Studio one at a time, each with
// the id of the model's tool use, and wait for the Studio's result; when the
// turn is stopped or the Studio goes away, every waiting call is answered
// "not run" and the turn ends (fail closed).

import type {
  CompanionMessage,
  Effective,
  StartOptions,
  ToolDefinition,
  Usage,
} from "./protocol.ts";
import { agentEnvironment, agentiqueTool, qualified, sdkOptions } from "./policy.ts";

/** What a tool call came back with. */
export interface ToolAnswer {
  content: string;
  isError: boolean;
}

/** Serves Agentique's tools to the SDK; every call goes to `call`. */
export type McpFactory = (
  tools: ToolDefinition[],
  call: (name: string, input: Record<string, unknown>, signal?: AbortSignal) => Promise<ToolAnswer>,
) => unknown;

/** The part of the SDK a turn uses (the real one, or a stand-in in tests). */
export interface Sdk {
  query(params: { prompt: AsyncIterable<unknown>; options: Record<string, unknown> }): QueryLike;
}

export interface QueryLike extends AsyncIterable<Record<string, unknown>> {
  interrupt(): Promise<unknown>;
}

/** A tool use's identity for matching: its name and canonical input. */
function key(name: string, input: unknown): string {
  return `${name}\u0000${canonical(input)}`;
}

/** JSON with object keys sorted, so equal inputs give equal text. */
export function canonical(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map(canonical).join(",")}]`;
  }
  if (value !== null && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>).sort(([a], [b]) =>
      a < b ? -1 : a > b ? 1 : 0,
    );
    return `{${entries.map(([k, v]) => `${JSON.stringify(k)}:${canonical(v)}`).join(",")}}`;
  }
  return JSON.stringify(value ?? null);
}

export class Turn {
  private readonly send: (message: CompanionMessage) => void;
  private readonly sdk: Sdk;
  private readonly mcp: McpFactory;
  private readonly from: Record<string, string | undefined>;
  private readonly client: string;
  private readonly abort = new AbortController();
  /** Tool uses the model asked for, waiting for their MCP call, by key. */
  private readonly uses = new Map<string, string[]>();
  /** Calls sent to the Studio, waiting for its answer. */
  private readonly waiting = new Map<string, (answer: ToolAnswer) => void>();
  /** One call at a time: each waits for the one before it. */
  private chain: Promise<unknown> = Promise.resolve();
  private next = 1;
  private query: QueryLike | null = null;
  private stopped: string | null = null;
  private finish: () => void = () => {};

  constructor(
    send: (message: CompanionMessage) => void,
    sdk: Sdk,
    mcp: McpFactory,
    from: Record<string, string | undefined>,
    client: string,
  ) {
    this.send = send;
    this.sdk = sdk;
    this.mcp = mcp;
    this.from = from;
    this.client = client;
  }

  /** Runs the turn to its end. Resolves once the SDK is done. */
  async run(start: StartOptions): Promise<void> {
    const names = start.tools.map((t) => t.name);
    const server = this.mcp(start.tools, (name, input, signal) =>
      this.call(name, input, names, signal),
    );
    const done = new Promise<void>((resolve) => {
      this.finish = resolve;
    });
    // The Operator's message, then the input stays open until the turn ends
    // (streaming input, so the turn can be interrupted).
    const prompt = (async function* () {
      yield {
        type: "user",
        message: { role: "user", content: start.prompt },
        parent_tool_use_id: null,
      };
      await done;
    })();
    const env = agentEnvironment(start, this.from, this.client);
    const options = sdkOptions(start, server, this.abort, env, (text) =>
      this.send({ type: "log", text: String(text).trimEnd() }),
    );
    try {
      this.query = this.sdk.query({ prompt, options: options as Record<string, unknown> });
      for await (const message of this.query) {
        if (this.handle(message)) {
          break;
        }
      }
    } catch (error) {
      if (this.stopped === null) {
        this.send({ type: "error", kind: "runtime", message: describe(error) });
      }
    } finally {
      this.release(this.stopped ?? "the turn ended");
      this.finish();
    }
    if (this.stopped !== null) {
      this.send({ type: "error", kind: "interrupted", message: this.stopped });
    }
  }

  /** The Studio's answer to a call. Unknown calls are ignored. */
  answer(call: string, answer: ToolAnswer): void {
    const resolve = this.waiting.get(call);
    if (resolve) {
      this.waiting.delete(call);
      resolve(answer);
    }
  }

  /** Stops the turn: waiting calls are not run, and the SDK is interrupted. */
  async stop(reason: string): Promise<void> {
    if (this.stopped !== null) {
      return;
    }
    this.stopped = reason;
    this.release(reason);
    try {
      await this.query?.interrupt();
    } catch {
      // The query may have ended already.
    }
    this.abort.abort();
    this.finish();
  }

  /** Answers every waiting call "not run". */
  private release(reason: string): void {
    for (const [call, resolve] of this.waiting) {
      resolve({ content: `Not run: ${reason}.`, isError: true });
      this.waiting.delete(call);
    }
  }

  /** One SDK message to the Studio. Returns true when the turn is over. */
  private handle(message: Record<string, unknown>): boolean {
    const type = message.type;
    const subtype = message.subtype;
    if (type === "system" && subtype === "init") {
      this.send({ type: "init", ...effective(message) });
    } else if (type === "system" && subtype === "api_retry") {
      const error = String(message.error ?? "unknown");
      this.send({
        type: "retry",
        attempt: Number(message.attempt ?? 0),
        error,
        status: typeof message.error_status === "number" ? message.error_status : null,
      });
      if (error === "authentication_failed") {
        // A refused key never succeeds on a retry: stop at once.
        this.send({
          type: "error",
          kind: "auth",
          message: "Anthropic refused the API key (authentication failed).",
        });
        this.abort.abort();
        return true;
      }
    } else if (type === "stream_event") {
      const event = message.event as Record<string, unknown> | undefined;
      const delta = event?.delta as Record<string, unknown> | undefined;
      if (event?.type === "content_block_delta" && delta) {
        if (delta.type === "text_delta" && typeof delta.text === "string") {
          this.send({ type: "text", text: delta.text });
        } else if (delta.type === "thinking_delta" && typeof delta.thinking === "string") {
          this.send({ type: "thinking", text: delta.thinking });
        }
      }
    } else if (type === "assistant" && message.parent_tool_use_id == null) {
      const inner = (message.message ?? {}) as Record<string, unknown>;
      const content = Array.isArray(inner.content) ? (inner.content as Record<string, unknown>[]) : [];
      for (const block of content) {
        if (block.type === "tool_use" && typeof block.id === "string") {
          const k = key(String(block.name), block.input);
          this.uses.set(k, [...(this.uses.get(k) ?? []), block.id]);
        }
      }
      if (message.error === "authentication_failed") {
        this.send({
          type: "error",
          kind: "auth",
          message: "Anthropic refused the API key (authentication failed).",
        });
        return true;
      }
      this.send({ type: "assistant", model: String(inner.model ?? ""), content });
    } else if (type === "result") {
      this.send({
        type: "result",
        isError: message.is_error === true,
        subtype: String(subtype ?? ""),
        stopReason: typeof message.stop_reason === "string" ? message.stop_reason : null,
        numTurns: Number(message.num_turns ?? 0),
        costUsd: typeof message.total_cost_usd === "number" ? message.total_cost_usd : null,
        usage: usage(message.usage),
        sessionId: String(message.session_id ?? ""),
        denials: Array.isArray(message.permission_denials)
          ? (message.permission_denials as Record<string, unknown>[]).map((d) =>
              String(d.tool_name ?? "?"),
            )
          : [],
        errors: Array.isArray(message.errors) ? (message.errors as unknown[]).map(String) : [],
      });
      return true;
    }
    return false;
  }

  /**
   * A tool call from the SDK's MCP server: only Agentique's tools, one at a
   * time, answered by the Studio; "not run" when the turn stops first.
   */
  private call(
    name: string,
    input: Record<string, unknown>,
    names: readonly string[],
    signal?: AbortSignal,
  ): Promise<ToolAnswer> {
    if (agentiqueTool(qualified(name), names) === null) {
      return Promise.resolve({ content: `Not run: ${name} is not one of Agentique's tools.`, isError: true });
    }
    const k = key(name, input);
    const ids = this.uses.get(k) ?? [];
    const toolUseId = ids.shift() ?? null;
    this.uses.set(k, ids);
    const result = this.chain.then(
      () =>
        new Promise<ToolAnswer>((resolve) => {
          if (this.stopped !== null || signal?.aborted) {
            resolve({ content: `Not run: ${this.stopped ?? "the call was cancelled"}.`, isError: true });
            return;
          }
          const call = `c${this.next++}`;
          this.waiting.set(call, resolve);
          signal?.addEventListener("abort", () =>
            this.answer(call, { content: "Not run: the call was cancelled.", isError: true }),
          );
          this.send({ type: "tool_call", call, toolUseId, name, input });
        }),
    );
    this.chain = result.catch(() => undefined);
    return result;
  }
}

/** The SDK's init message as what the agent can actually do. */
export function effective(message: Record<string, unknown>): Effective {
  const strings = (value: unknown) => (Array.isArray(value) ? value.map((v) => (typeof v === "string" ? v : String((v as Record<string, unknown>)?.name ?? v))) : []);
  return {
    sessionId: String(message.session_id ?? ""),
    model: String(message.model ?? ""),
    claudeCode: String(message.claude_code_version ?? ""),
    tools: strings(message.tools),
    mcpServers: Array.isArray(message.mcp_servers)
      ? (message.mcp_servers as Record<string, unknown>[]).map((s) => ({
          name: String(s.name ?? ""),
          status: String(s.status ?? ""),
        }))
      : [],
    permissionMode: String(message.permissionMode ?? ""),
    apiKeySource: String(message.apiKeySource ?? ""),
    skills: strings(message.skills),
    agents: strings(message.agents),
    plugins: strings(message.plugins),
  };
}

function usage(value: unknown): Usage | null {
  if (value === null || typeof value !== "object") {
    return null;
  }
  const u = value as Record<string, unknown>;
  const n = (v: unknown) => (typeof v === "number" ? v : 0);
  return {
    inputTokens: n(u.input_tokens),
    outputTokens: n(u.output_tokens),
    cacheReadTokens: n(u.cache_read_input_tokens),
    cacheWriteTokens: n(u.cache_creation_input_tokens),
  };
}

function describe(error: unknown): string {
  const text = error instanceof Error ? error.message : String(error);
  return text.length > 2000 ? `${text.slice(0, 2000)}…` : text;
}
