// Protocol 2 between the Studio and the Claude Agent companion (ROADMAP
// §4.7, C-53): one JSON object per line on standard input and output, UTF-8.
// The Studio starts one companion per turn; it answers `ready`, then the
// Studio sends `start` and later `tool_result`, `permission_result`,
// `message`, `gate`, `interrupt` or `close`. Everything the companion says
// goes to standard output; standard error is diagnostics only. Any line that
// is not a known message ends the turn with a protocol error: the companion
// never guesses. C-54 adds optional fields only: the credential the SDK
// uses in `init`, the `source` of an `auth` error when it would have used
// another, and a session's spend ceiling in `start`.

export const PROTOCOL = 2;

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

/** A command the policy refuses, and why (shown to the agent). */
export interface RefusedCommand {
  /** A JavaScript regular expression, matched case-insensitively. */
  pattern: string;
  reason: string;
}

/**
 * What a development session may do (C-53): the Studio's decision, enforced
 * in the companion's one pre-tool hook. Paths are absolute folders; the
 * patterns in `protected` and `hidden` are relative to them (`*` matches
 * within a name, `**` across folders, a plain path covers what is under it).
 */
export interface Policy {
  /** Folders the file tools may read. */
  read: string[];
  /** Folders the file tools may write; each inside a read folder. */
  write: string[];
  /** Never written by a file tool (the model files are always among them). */
  protected: string[];
  /** Never read by a file tool (keys and other secrets). */
  hidden: string[];
  /** Whether commands may run at all (trusted-local execution). */
  commands: boolean;
  /** Commands refused even then. */
  refusedCommands: RefusedCommand[];
  /** Web fetch and search. */
  network: boolean;
  /** MCP servers besides Agentique's whose tools may run. */
  mcpServers: string[];
  /** A call the policy does not decide: ask the Studio, or refuse it. */
  undecided: "ask" | "refuse";
}

/** An Anthropic-compatible endpoint, such as DeepSeek's (C-53). */
export interface Endpoint {
  baseUrl: string;
  /** The model the SDK uses for its own small tasks; null for the main one. */
  fastModel: string | null;
}

/** A subagent the session may delegate to (the SDK's AgentDefinition). */
export interface AgentDefinition {
  description: string;
  prompt: string;
  tools?: string[];
  disallowedTools?: string[];
  model?: string;
}

/** What one turn runs with. Every field is the Studio's decision. */
export interface StartOptions {
  /** The Operator's message for this turn. */
  prompt: string;
  /** Agentique's own instructions (its skills). */
  systemPrompt: string;
  tools: ToolDefinition[];
  /** The model id, or null for the SDK's default. */
  model: string | null;
  effort: "low" | "medium" | "high" | "xhigh" | "max" | null;
  /** The SDK session to resume, or null for a new one. */
  resume: string | null;
  /** A bound on the turn's model calls (the SDK's maxTurns). */
  maxTurns: number;
  /** The agent's working folder. */
  cwd: string;
  /** The SDK's configuration folder (CLAUDE_CONFIG_DIR): its sessions. */
  configDir: string;
  /** What the agent's process sees as its home folder (no policy only). */
  home: string;
  /**
   * The development policy, or null for a session with Agentique's tools
   * only and nothing of the machine (as in protocol 1).
   */
  policy: Policy | null;
  /** The project's settings to load (`project`, `local`); empty for none. */
  settingSources: ("project" | "local")[];
  /** Subagents besides the SDK's own and the project's. */
  agents: Record<string, AgentDefinition>;
  /** An Anthropic-compatible endpoint, or null for Anthropic's API. */
  endpoint: Endpoint | null;
  /**
   * Whether Agentique's instructions are appended to the SDK's own
   * development instructions (true) or are the whole system prompt.
   */
  preset: boolean;
  /**
   * A spend ceiling for the session in US dollars (C-54), which the SDK
   * enforces with its own estimate at Claude's prices; the Studio gives one
   * only for Anthropic's API. Absent or null: none.
   */
  maxBudgetUsd?: number | null;
}

export type GateMode = "run" | "pause" | "step";

export type HostMessage =
  | { type: "start"; options: StartOptions }
  | { type: "tool_result"; call: string; content: string; isError: boolean }
  | { type: "permission_result"; call: string; allow: boolean; message: string }
  | { type: "message"; text: string }
  | { type: "gate"; mode: GateMode }
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
  /**
   * The credential the SDK uses (C-54), as it reported it before the first
   * model call: where its key comes from (`ANTHROPIC_API_KEY` for a given
   * key; `none` with the subscription token), its API (`firstParty` is
   * Anthropic's, also through an Anthropic-compatible endpoint) and its
   * token's source (`CLAUDE_CODE_OAUTH_TOKEN` for the given subscription
   * token). Never an email or an organisation.
   */
  apiKeySource: string;
  apiProvider: string;
  tokenSource: string;
  skills: string[];
  agents: string[];
  plugins: string[];
}

/**
 * Whether this computer has a Claude login (C-54), from `claude auth
 * status`: only whether and how, never its token, email or organisation.
 * Agentique never uses it; Settings say why.
 */
export interface Login {
  loggedIn: boolean;
  /** `none`, `claude.ai`, `oauth_token`, `api_key`, `api_key_helper` or `third_party`. */
  authMethod: string;
  apiProvider: string | null;
  /** The claude.ai plan (`max`, `pro`, …), when it is one. */
  subscriptionType: string | null;
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
  | { type: "assistant"; id: string; model: string; content: unknown[] }
  | {
      type: "tool_call";
      call: string;
      toolUseId: string | null;
      name: string;
      input: Record<string, unknown>;
    }
  | { type: "tool_done"; toolUseId: string; isError: boolean; content: string }
  | { type: "permission"; call: string; tool: string; input: Record<string, unknown>; reason: string }
  | {
      type: "task";
      event: "started" | "progress" | "done";
      id: string;
      description: string;
      agent: string | null;
      status: string | null;
      summary: string | null;
    }
  | { type: "compaction"; trigger: string; preTokens: number; postTokens: number | null }
  | { type: "paused"; tool: string }
  /** The turn is over: nothing more is taken from the Studio. */
  | { type: "done" }
  /** A message that arrived after the turn was over, not given to it. */
  | { type: "undelivered"; text: string }
  | { type: "retry"; attempt: number; error: string; status: number | null }
  | {
      type: "result";
      isError: boolean;
      subtype: string;
      stopReason: string | null;
      numTurns: number;
      costUsd: number | null;
      /** Tokens since the turn's previous result, all models together. */
      usage: Usage | null;
      /** The same, by model. */
      usageByModel: Record<string, Usage> | null;
      sessionId: string;
      denials: string[];
      errors: string[];
    }
  /**
   * `source` is set when the SDK would have used another credential than
   * the key the Studio gave (`kind` is then `auth`): the source it named.
   */
  | { type: "error"; kind: ErrorKind; message: string; source?: string }
  | { type: "log"; text: string };

/**
 * Why a turn could not go on: the key was refused or another credential
 * would have been used, a Claude plan's usage limit was reached, the runtime
 * failed, the protocol was broken, or the Studio stopped it.
 */
export type ErrorKind = "auth" | "limit" | "runtime" | "protocol" | "interrupted";

export function encode(message: CompanionMessage): string {
  return JSON.stringify(message) + "\n";
}

function strings(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

/** Checks a policy's shape; throws with a plain reason. */
export function checkPolicy(value: unknown): Policy {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("`start` options.policy must be an object or null");
  }
  const p = value as Record<string, unknown>;
  for (const field of ["read", "write", "protected", "hidden", "mcpServers"]) {
    if (!strings(p[field])) {
      throw new Error(`options.policy.${field} must be a list of strings`);
    }
  }
  for (const field of ["commands", "network"]) {
    if (typeof p[field] !== "boolean") {
      throw new Error(`options.policy.${field} must be true or false`);
    }
  }
  if (p.undecided !== "ask" && p.undecided !== "refuse") {
    throw new Error("options.policy.undecided must be `ask` or `refuse`");
  }
  if (
    !Array.isArray(p.refusedCommands) ||
    !p.refusedCommands.every(
      (r) =>
        typeof r === "object" &&
        r !== null &&
        typeof (r as RefusedCommand).pattern === "string" &&
        typeof (r as RefusedCommand).reason === "string",
    )
  ) {
    throw new Error("options.policy.refusedCommands must be a list of {pattern, reason}");
  }
  for (const r of p.refusedCommands as RefusedCommand[]) {
    try {
      new RegExp(r.pattern, "i");
    } catch {
      throw new Error(`options.policy.refusedCommands has an invalid pattern ${JSON.stringify(r.pattern)}`);
    }
  }
  return p as unknown as Policy;
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
      if (options.policy !== null) {
        checkPolicy(options.policy);
      }
      if (!strings(options.settingSources) || options.settingSources.some((s) => s !== "project" && s !== "local")) {
        throw new Error("`start` options.settingSources must list `project` or `local`");
      }
      if (typeof options.agents !== "object" || options.agents === null || Array.isArray(options.agents)) {
        throw new Error("`start` options.agents must be an object");
      }
      if (options.endpoint !== null) {
        const e = options.endpoint as Record<string, unknown> | undefined;
        if (!e || typeof e.baseUrl !== "string" || !/^https?:\/\//.test(e.baseUrl)) {
          throw new Error("`start` options.endpoint needs an http(s) baseUrl");
        }
      }
      if (typeof options.preset !== "boolean") {
        throw new Error("`start` options.preset must be true or false");
      }
      const ceiling = options.maxBudgetUsd;
      if (ceiling !== undefined && ceiling !== null && !(typeof ceiling === "number" && Number.isFinite(ceiling) && ceiling > 0)) {
        throw new Error("`start` options.maxBudgetUsd must be a positive number or null");
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
    case "permission_result":
      if (typeof message.call !== "string" || typeof message.allow !== "boolean") {
        throw new Error("`permission_result` needs a call and allow");
      }
      return {
        type: "permission_result",
        call: message.call,
        allow: message.allow,
        message: typeof message.message === "string" ? message.message : "",
      };
    case "message":
      if (typeof message.text !== "string" || message.text.trim() === "") {
        throw new Error("`message` needs text");
      }
      return { type: "message", text: message.text };
    case "gate":
      if (message.mode !== "run" && message.mode !== "pause" && message.mode !== "step") {
        throw new Error("`gate` needs mode run, pause or step");
      }
      return { type: "gate", mode: message.mode };
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
