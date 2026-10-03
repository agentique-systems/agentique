// Protocol 1 between the Studio and the Claude Agent companion (ROADMAP
// §4.7): one JSON object per line on standard input and output, UTF-8. The
// Studio starts one companion per turn; it answers `ready`, then the Studio
// sends `start` and later `tool_result`, `interrupt` or `close`. Everything
// the companion says goes to standard output; standard error is
// diagnostics only. Any line that is not a known message ends the turn
// with a protocol error: the companion never guesses.

export const PROTOCOL = 1;

/** Lines longer than this are refused (a tool result is capped far below). */
export const MAX_LINE = 4 * 1024 * 1024;

/** One of Agentique's tools, as the Studio defines it. */
export interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  /** Whether the tool only reads (an MCP annotation for the model). */
  readOnly?: boolean;
}

/** What one turn runs with. Every field is the Studio's decision. */
export interface StartOptions {
  /** The Operator's message for this turn. */
  prompt: string;
  /** Agentique's own instructions (its skills), the whole system prompt. */
  systemPrompt: string;
  tools: ToolDefinition[];
  /** The model id, or null for the SDK's default. */
  model: string | null;
  effort: "low" | "medium" | "high" | "xhigh" | "max" | null;
  /** The SDK session to resume, or null for a new one. */
  resume: string | null;
  /** A bound on the turn's model calls (the SDK's maxTurns). */
  maxTurns: number;
  /** The agent's working folder: empty, Agentique's own. */
  cwd: string;
  /** The SDK's configuration folder (CLAUDE_CONFIG_DIR): its sessions. */
  configDir: string;
  /** What the agent's process sees as its home folder. */
  home: string;
}

export type HostMessage =
  | { type: "start"; options: StartOptions }
  | { type: "tool_result"; call: string; content: string; isError: boolean }
  | { type: "interrupt" }
  | { type: "close" };

/** The configuration the SDK reported when it started: what it can do. */
export interface Effective {
  sessionId: string;
  model: string;
  claudeCode: string;
  tools: string[];
  mcpServers: { name: string; status: string }[];
  permissionMode: string;
  apiKeySource: string;
  skills: string[];
  agents: string[];
  plugins: string[];
}

export interface Usage {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
}

export type CompanionMessage =
  | { type: "ready"; protocol: number; sdk: string; claudeCode: string; node: string }
  | ({ type: "init" } & Effective)
  | { type: "text"; text: string }
  | { type: "thinking"; text: string }
  | { type: "assistant"; model: string; content: unknown[] }
  | {
      type: "tool_call";
      call: string;
      toolUseId: string | null;
      name: string;
      input: Record<string, unknown>;
    }
  | { type: "retry"; attempt: number; error: string; status: number | null }
  | {
      type: "result";
      isError: boolean;
      subtype: string;
      stopReason: string | null;
      numTurns: number;
      costUsd: number | null;
      usage: Usage | null;
      sessionId: string;
      denials: string[];
      errors: string[];
    }
  | { type: "error"; kind: ErrorKind; message: string }
  | { type: "log"; text: string };

/**
 * Why a turn could not go on: the key was refused, the runtime failed, the
 * protocol was broken, or the Studio stopped it.
 */
export type ErrorKind = "auth" | "runtime" | "protocol" | "interrupted";

export function encode(message: CompanionMessage): string {
  return JSON.stringify(message) + "\n";
}

/** Parses one line from the Studio. Throws with a plain reason. */
export function decode(line: string): HostMessage {
  if (line.length > MAX_LINE) {
    throw new Error(`a line of ${line.length} characters is over the limit`);
  }
  const value: unknown = JSON.parse(line);
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("a message must be a JSON object");
  }
  const message = value as Record<string, unknown>;
  switch (message.type) {
    case "start": {
      const options = message.options as Record<string, unknown> | undefined;
      if (!options || typeof options.prompt !== "string" || !Array.isArray(options.tools)) {
        throw new Error("`start` needs options with a prompt and tools");
      }
      for (const field of ["cwd", "configDir", "home", "systemPrompt"]) {
        if (typeof options[field] !== "string" || options[field] === "") {
          throw new Error(`\`start\` needs options.${field}`);
        }
      }
      return message as unknown as HostMessage;
    }
    case "tool_result":
      if (typeof message.call !== "string" || typeof message.content !== "string") {
        throw new Error("`tool_result` needs a call and content");
      }
      return {
        type: "tool_result",
        call: message.call,
        content: message.content,
        isError: message.isError === true,
      };
    case "interrupt":
    case "close":
      return { type: message.type };
    default:
      throw new Error(`unknown message type ${JSON.stringify(message.type)}`);
  }
}

/** Splits a stream of text into lines, refusing over-long ones. */
export class LineSplitter {
  private buffer = "";
  push(chunk: string): string[] {
    this.buffer += chunk;
    const lines = this.buffer.split("\n");
    this.buffer = lines.pop() ?? "";
    if (this.buffer.length > MAX_LINE) {
      throw new Error("a line is over the limit");
    }
    return lines.map((l) => l.replace(/\r$/, "")).filter((l) => l.trim() !== "");
  }
}
