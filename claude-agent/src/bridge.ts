// One turn of the Claude Agent runtime (ROADMAP §4.7, §4.10): the SDK runs
// the loop; this file only carries what happens to the Studio and the
// Studio's answers back. Calls of Agentique's tools go to the Studio one at a
// time, each with the id of the model's tool use, and wait for the Studio's
// result; a call the permission policy leaves undecided is asked of the
// Studio the same way. When the turn is stopped or the Studio goes away,
// every waiting call is answered "not run" and the turn ends (fail closed).
//
// Protocol 2 adds (C-53): messages the Studio queues into the running
// session, the pause gate (Pause holds the session at its next tool call,
// Step lets one through), and reports of the SDK's own tool results,
// subagent tasks and compaction, so the Studio shows what the agent does.

import type {
  CompanionMessage,
  Effective,
  GateMode,
  StartOptions,
  ToolDefinition,
  Usage,
} from "./protocol.ts";
import { type HookHost, agentEnvironment, agentiqueTool, qualified, sdkOptions } from "./policy.ts";

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

/** How much of a built-in tool's result the Studio is shown. */
export const TOOL_DONE_LIMIT = 4000;

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

/** A tool result's content as plain text, cut to `limit` characters. */
export function resultText(content: unknown, limit = TOOL_DONE_LIMIT): string {
  let text: string;
  if (typeof content === "string") {
    text = content;
  } else if (Array.isArray(content)) {
    text = content
      .map((block) => {
        const b = block as Record<string, unknown>;
        return b.type === "text" && typeof b.text === "string" ? b.text : `[${String(b.type ?? "content")}]`;
      })
      .join("\n");
  } else {
    text = content === undefined || content === null ? "" : JSON.stringify(content);
  }
  return text.length > limit ? `${text.slice(0, limit)}…` : text;
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
  /** Calls and questions sent to the Studio, waiting for its answer. */
  private readonly waiting = new Map<string, (answer: ToolAnswer) => void>();
  private readonly asking = new Map<string, (answer: { allow: boolean; message: string }) => void>();
  /** One call at a time: each waits for the one before it. */
  private chain: Promise<unknown> = Promise.resolve();
  private next = 1;
  private query: QueryLike | null = null;
  private stopped: string | null = null;
  private finish: () => void = () => {};
  /** Messages for the session not yet given to it, and who waits for one. */
  private readonly queued: string[] = [];
  private wake: () => void = () => {};
  /** User messages given to the session, and results it reported. */
  private given = 0;
  private results = 0;
  /** The pause gate, and the calls held at it. */
  private mode: GateMode = "run";
  private readonly held: (() => void)[] = [];

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
    // The Operator's message, then any message the Studio queues, while the
    // input stays open until the turn ends (streaming input, so the turn can
    // be interrupted and steered).
    const turn = this;
    const prompt = (async function* () {
      turn.given += 1;
      yield { type: "user", message: { role: "user", content: start.prompt }, parent_tool_use_id: null };
      for (;;) {
        while (turn.queued.length > 0) {
          const text = turn.queued.shift() as string;
          turn.given += 1;
          yield { type: "user", message: { role: "user", content: text }, parent_tool_use_id: null };
        }
        const woken = new Promise<void>((resolve) => {
          turn.wake = resolve;
        });
        const ended = await Promise.race([done.then(() => true), woken.then(() => false)]);
        if (ended) {
          return;
        }
      }
    })();
    const env = agentEnvironment(start, this.from, this.client);
    const host: HookHost = {
      gate: (tool) => this.gate(tool),
      ask: (tool, input, reason, signal) => this.ask(tool, input, reason, signal),
    };
    const options = sdkOptions(start, server, this.abort, env, (text) =>
      this.send({ type: "log", text: String(text).trimEnd() }), host,
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

  /** The Studio's answer to a permission question. Unknown ones are ignored. */
  permit(call: string, allow: boolean, message: string): void {
    const resolve = this.asking.get(call);
    if (resolve) {
      this.asking.delete(call);
      resolve({ allow, message });
    }
  }

  /** A message the Studio queues into the running session. */
  queue(text: string): void {
    if (this.stopped !== null) {
      return;
    }
    this.queued.push(text);
    this.wake();
  }

  /** Pause holds the session at its next tool call; Step lets one through. */
  setGate(mode: GateMode): void {
    if (mode === "run") {
      this.mode = "run";
      for (const resume of this.held.splice(0)) {
        resume();
      }
    } else if (mode === "step") {
      // One call goes on: one already held, or else the next to arrive.
      const first = this.held.shift();
      if (first) {
        this.mode = "pause";
        first();
      } else {
        this.mode = "step";
      }
    } else {
      this.mode = "pause";
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

  /** Waits at the gate while paused; never once stopped. */
  private gate(tool: string): Promise<void> {
    if (this.mode === "run" || this.stopped !== null) {
      return Promise.resolve();
    }
    if (this.mode === "step") {
      this.mode = "pause";
      return Promise.resolve();
    }
    this.send({ type: "paused", tool });
    return new Promise<void>((resolve) => this.held.push(resolve));
  }

  /** Asks the Studio about a call the policy leaves undecided. */
  private ask(
    tool: string,
    input: Record<string, unknown>,
    reason: string,
    signal?: AbortSignal,
  ): Promise<{ allow: boolean; message: string }> {
    if (this.stopped !== null || signal?.aborted) {
      return Promise.resolve({ allow: false, message: `Not run: ${this.stopped ?? "the call was cancelled"}.` });
    }
    const call = `p${this.next++}`;
    return new Promise((resolve) => {
      this.asking.set(call, resolve);
      signal?.addEventListener("abort", () => this.permit(call, false, "The call was cancelled."));
      this.send({ type: "permission", call, tool, input, reason });
    });
  }

  /** Answers every waiting call and question "not run", and opens the gate. */
  private release(reason: string): void {
    for (const [call, resolve] of this.waiting) {
      resolve({ content: `Not run: ${reason}.`, isError: true });
      this.waiting.delete(call);
    }
    for (const [call, resolve] of this.asking) {
      resolve({ allow: false, message: `Not run: ${reason}.` });
      this.asking.delete(call);
    }
    for (const resume of this.held.splice(0)) {
      resume();
    }
  }

  /** One SDK message to the Studio. Returns true when the turn is over. */
  private handle(message: Record<string, unknown>): boolean {
    const type = message.type;
    const subtype = message.subtype;
    const top = message.parent_tool_use_id == null;
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
          message: "The model's API refused the key (authentication failed).",
        });
        this.abort.abort();
        return true;
      }
    } else if (type === "system" && subtype === "compact_boundary") {
      const meta = (message.compact_metadata ?? {}) as Record<string, unknown>;
      this.send({
        type: "compaction",
        trigger: String(meta.trigger ?? "auto"),
        preTokens: Number(meta.pre_tokens ?? 0),
        postTokens: typeof meta.post_tokens === "number" ? meta.post_tokens : null,
      });
    } else if (type === "system" && (subtype === "task_started" || subtype === "task_progress" || subtype === "task_notification")) {
      if (message.ambient === true || message.skip_transcript === true) {
        return false;
      }
      this.send({
        type: "task",
        event: subtype === "task_started" ? "started" : subtype === "task_progress" ? "progress" : "done",
        id: String(message.task_id ?? ""),
        description: String(message.description ?? message.summary ?? ""),
        agent: typeof message.subagent_type === "string" ? message.subagent_type : null,
        status: typeof message.status === "string" ? message.status : null,
        summary: typeof message.summary === "string" ? message.summary : null,
      });
    } else if (type === "stream_event") {
      if (!top) {
        return false;
      }
      const event = message.event as Record<string, unknown> | undefined;
      const delta = event?.delta as Record<string, unknown> | undefined;
      if (event?.type === "content_block_delta" && delta) {
        if (delta.type === "text_delta" && typeof delta.text === "string") {
          this.send({ type: "text", text: delta.text });
        } else if (delta.type === "thinking_delta" && typeof delta.thinking === "string") {
          this.send({ type: "thinking", text: delta.thinking });
        }
      }
    } else if (type === "assistant" && top) {
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
          message: "The model's API refused the key (authentication failed).",
        });
        return true;
      }
      this.send({ type: "assistant", model: String(inner.model ?? ""), content });
    } else if (type === "user" && top) {
      // The SDK's own tools' results (Agentique's are the Studio's already).
      const inner = (message.message ?? {}) as Record<string, unknown>;
      const content = Array.isArray(inner.content) ? (inner.content as Record<string, unknown>[]) : [];
      for (const block of content) {
        if (block.type === "tool_result" && typeof block.tool_use_id === "string") {
          this.send({
            type: "tool_done",
            toolUseId: block.tool_use_id,
            isError: block.is_error === true,
            content: resultText(block.content),
          });
        }
      }
    } else if (type === "result") {
      this.results += 1;
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
      // A queued message gets its own result; the turn ends after the last.
      return this.results >= this.given && this.queued.length === 0;
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
