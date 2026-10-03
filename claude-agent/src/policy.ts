// What the agent may do, decided here and nowhere else in the companion
// (ROADMAP §4.7, C-53). Two kinds of session:
//
// - Without a policy (protocol 1's only kind): the SDK's own tools are all
//   off, the agent sees only Agentique's tools, nothing of the machine's
//   configuration is loaded, and the environment is built from nothing.
// - With a development policy: the SDK's own tools work under the Studio's
//   permission policy, enforced in one pre-tool hook (`decide`): what the
//   policy allows runs without asking anybody, what it forbids is refused
//   with the reason, and anything else is asked of the Studio (or refused).
//   A model file is never written with a file tool, so the model changes
//   only through Agentique's tools. The project's configuration is loaded,
//   never the machine's user settings, and the environment carries no key
//   but the model's.
//
// These are gates on what the agent can do; they are not an operating-system
// sandbox, and the Studio's own checks still decide every model change.

import { existsSync, realpathSync } from "node:fs";
import type { Policy, StartOptions } from "./protocol.ts";

/** The MCP server's name: tools are `mcp__agentique__<name>`. */
export const SERVER = "agentique";

/**
 * Claude Code's built-in tools, named so they are removed from the agent's
 * context as well as left out (`tools: []` already leaves them out).
 */
export const BUILT_IN_TOOLS = [
  "Agent",
  "AskUserQuestion",
  "Bash",
  "BashOutput",
  "Edit",
  "ExitPlanMode",
  "Glob",
  "Grep",
  "KillShell",
  "ListMcpResourcesTool",
  "MultiEdit",
  "NotebookEdit",
  "Read",
  "ReadMcpResourceTool",
  "Skill",
  "SlashCommand",
  "Task",
  "TodoWrite",
  "WebFetch",
  "WebSearch",
  "Write",
];

/**
 * Built-in tools a development session never gets: worktrees, schedules and
 * remote triggers are the Orchestrator's and the Studio's, and questions to
 * the Operator go through Agentique's `ask_operator`.
 */
export const DEVELOPMENT_DISALLOWED = [
  "AskUserQuestion",
  "CronCreate",
  "CronDelete",
  "CronList",
  "EnterWorktree",
  "ExitWorktree",
  "RemoteTrigger",
  "ScheduleWakeup",
];

/** Built-in tools that touch nothing outside the session itself. */
const SESSION_TOOLS = new Set([
  "Agent",
  "Task",
  "TaskOutput",
  "TaskStop",
  "KillShell",
  "BashOutput",
  "TodoWrite",
  "Skill",
  "ToolSearch",
  "EnterPlanMode",
  "ExitPlanMode",
  "ListMcpResourcesTool",
  "ReadMcpResourceTool",
  "ReadMcpResourceDir",
  "RefreshMcpTools",
  "SendMessage",
  "ListAgents",
  "ReportFindings",
  "TaskCreate",
  "TaskGet",
  "TaskUpdate",
  "TaskList",
]);

const READ_TOOLS: Record<string, string[]> = {
  Read: ["file_path"],
  Glob: ["path"],
  Grep: ["path"],
  LS: ["path"],
  NotebookRead: ["notebook_path"],
};

const WRITE_TOOLS: Record<string, string> = {
  Write: "file_path",
  Edit: "file_path",
  MultiEdit: "file_path",
  NotebookEdit: "notebook_path",
};

const COMMAND_TOOLS = new Set(["Bash", "PowerShell"]);

/** The full name the agent calls a tool by. */
export function qualified(tool: string): string {
  return `mcp__${SERVER}__${tool}`;
}

/** The tool's own name if `name` is one of Agentique's tools, else null. */
export function agentiqueTool(name: string, allowed: readonly string[]): string | null {
  const prefix = `mcp__${SERVER}__`;
  if (!name.startsWith(prefix)) {
    return null;
  }
  const tool = name.slice(prefix.length);
  return allowed.includes(tool) ? tool : null;
}

// Paths: read the same way on every host (`/` and `\` both separate), so the
// policy's tests mean the same on Windows and Linux.

/** A Windows verbatim path (`\\?\C:\x`, `\\?\UNC\s\x`) in its ordinary form. */
function verbatimless(path: string): string {
  const unc = /^[\\/]{2}\?[\\/]UNC[\\/]/i;
  if (unc.test(path)) {
    return `\\\\${path.replace(unc, "")}`;
  }
  return path.replace(/^[\\/]{2}\?[\\/]/, "");
}

/**
 * The path as the file system resolves it: the nearest part of it that
 * exists, through links, junctions and short (8.3) names, with the rest
 * after it. A path that cannot be resolved is left as written.
 */
export function real(path: string, cwd: string): string {
  const written = normalise(path, cwd);
  const parts = written.split("/");
  for (let end = parts.length; end > 1; end--) {
    const head = parts.slice(0, end).join("/") || "/";
    try {
      if (existsSync(head)) {
        const resolved = normalise(realpathSync.native(head), cwd);
        return [resolved, ...parts.slice(end)].join("/").replace(/\/+$/, "") || resolved;
      }
    } catch {
      return written;
    }
  }
  return written;
}

function isAbsolute(path: string): boolean {
  return /^[A-Za-z]:[\\/]/.test(path) || path.startsWith("/") || path.startsWith("\\");
}

/** Whether paths compare without case (Windows drive paths and Windows). */
function foldsCase(path: string): boolean {
  return /^[A-Za-z]:/.test(path) || process.platform === "win32";
}

/**
 * `path` (relative to `cwd` unless absolute) as one canonical text: `/`
 * separators, `.` and `..` resolved, lowercase where the host ignores case.
 * A `..` above the root stays at the root.
 */
export function normalise(path: string, cwd: string): string {
  const full = verbatimless(isAbsolute(path) ? path : `${cwd}/${path}`);
  const drive = /^[A-Za-z]:/.test(full) ? full.slice(0, 2) : "";
  const parts: string[] = [];
  for (const part of full.slice(drive.length).split(/[\\/]+/)) {
    if (part === "" || part === ".") {
      continue;
    }
    if (part === "..") {
      parts.pop();
    } else {
      parts.push(part);
    }
  }
  const text = `${drive}/${parts.join("/")}`;
  return foldsCase(full) ? text.toLowerCase() : text;
}

/** Whether `path` is `root` or inside it (both normalised). */
function inside(path: string, root: string): boolean {
  return path === root || path.startsWith(root.endsWith("/") ? root : `${root}/`);
}

/** `path` relative to the first of `roots` it is inside, or null. */
function relativeTo(path: string, roots: string[], cwd: string): string | null {
  for (const root of roots) {
    for (const r of [normalise(root, cwd), real(root, cwd)]) {
      if (inside(path, r)) {
        return path.slice(r.length).replace(/^\//, "");
      }
    }
  }
  return null;
}

/** A name Windows reads differently from how it is written: a stream
 * (`a:b`), or a trailing dot or space. */
function trickyName(path: string): string | null {
  const parts = path.replace(/^[A-Za-z]:/, "").split(/[\\/]+/);
  return parts.find((p) => p.includes(":") || /[. ]$/.test(p)) ?? null;
}

/** Whether a relative path matches a policy pattern (see `Policy`). */
export function matches(relative: string, pattern: string): boolean {
  const p = pattern.replace(/\\/g, "/").replace(/^\/+|\/+$/g, "").toLowerCase();
  const r = relative.toLowerCase();
  if (!/[*?]/.test(p)) {
    return r === p || r.startsWith(`${p}/`);
  }
  const anywhere = p.startsWith("**/");
  const body = anywhere ? p.slice(3) : p;
  const source = body
    .split("**")
    .map((piece) =>
      piece
        .split("*")
        .map((s) => s.replace(/[.+^${}()|[\]\\?]/g, "\\$&"))
        .join("[^/]*"),
    )
    .join(".*");
  return new RegExp(`^${anywhere ? "(.*/)?" : ""}${source}(/.*)?$`).test(r);
}

/** What the policy says about one call. */
export type Decision =
  | { kind: "allow" }
  | { kind: "deny"; reason: string }
  | { kind: "ask"; reason: string };

const allow: Decision = { kind: "allow" };

function hiddenOrOutside(path: string, policy: Policy, cwd: string): Decision {
  const relative = relativeTo(path, policy.read, cwd);
  if (relative === null) {
    return { kind: "ask", reason: `${path} is outside the folders this session may read` };
  }
  const hidden = policy.hidden.find((h) => matches(relative, h));
  if (hidden !== undefined) {
    return { kind: "deny", reason: `${relative} is not readable in this session (it can hold keys or other secrets)` };
  }
  return allow;
}

/**
 * The policy's decision for one tool call. `tools` are Agentique's own (the
 * Studio checks and carries them out, so they are always allowed here).
 */
export function decide(
  tool: string,
  input: Record<string, unknown>,
  policy: Policy,
  cwd: string,
  tools: readonly string[],
): Decision {
  if (agentiqueTool(tool, tools) !== null) {
    return allow;
  }
  if (DEVELOPMENT_DISALLOWED.includes(tool)) {
    return { kind: "deny", reason: `${tool} is not available in Agentique` + (tool === "AskUserQuestion" ? "; ask with ask_operator" : "") };
  }
  if (tool in READ_TOOLS) {
    for (const field of READ_TOOLS[tool]) {
      const value = input[field];
      const path = real(typeof value === "string" && value !== "" ? value : ".", cwd);
      const decision = hiddenOrOutside(path, policy, cwd);
      if (decision.kind !== "allow") {
        return decision;
      }
    }
    // A search over hidden files is refused by the deny rules the SDK is
    // given (they become ripgrep exclusions); a glob naming one is refused
    // here too.
    const pattern = typeof input.glob === "string" ? input.glob : typeof input.pattern === "string" && tool === "Glob" ? input.pattern : "";
    if (pattern && policy.hidden.some((h) => matches(pattern.replace(/^\.\//, ""), h) || pattern.includes(h.replace(/^\*\*\//, "")))) {
      return { kind: "deny", reason: `${pattern} names files that can hold keys; they are not read in this session` };
    }
    return allow;
  }
  if (tool in WRITE_TOOLS) {
    const value = input[WRITE_TOOLS[tool]];
    if (typeof value !== "string" || value === "") {
      return { kind: "deny", reason: `${tool} needs a file path` };
    }
    const tricky = trickyName(value);
    if (tricky !== null) {
      return { kind: "deny", reason: `${value}: the name ${tricky} is read differently by Windows; write the plain path` };
    }
    const path = real(value, cwd);
    const relative = relativeTo(path, policy.write, cwd);
    if (relative === null) {
      return { kind: "deny", reason: `${value} is outside the folders this session may write` };
    }
    if (policy.protected.some((p) => matches(relative, p))) {
      return /(^|\/)model\//.test(relative)
        ? { kind: "deny", reason: `${relative} is part of the model: change the model with Agentique's tools (apply_changes), never by editing its files` }
        : { kind: "deny", reason: `${relative} is protected in this session` };
    }
    return hiddenOrOutside(path, policy, cwd);
  }
  if (COMMAND_TOOLS.has(tool)) {
    if (!policy.commands) {
      return { kind: "deny", reason: "Commands are off for this project (trusted-local execution is not on)" };
    }
    const command = typeof input.command === "string" ? input.command : "";
    for (const refused of policy.refusedCommands) {
      if (new RegExp(refused.pattern, "i").test(command)) {
        return { kind: "deny", reason: refused.reason };
      }
    }
    return allow;
  }
  if (tool === "WebFetch" || tool === "WebSearch") {
    return policy.network ? allow : { kind: "deny", reason: "The network is off for this session" };
  }
  if (tool === "Monitor") {
    // Monitor runs a command or watches a socket: the same checks as those.
    if (typeof input.command === "string" && input.command !== "") {
      const decision = decide("Bash", { command: input.command }, policy, cwd, tools);
      if (decision.kind !== "allow") {
        return decision;
      }
    }
    if ((typeof input.ws === "string" && input.ws !== "") || (typeof input.url === "string" && input.url !== "")) {
      if (!policy.network) {
        return { kind: "deny", reason: "The network is off for this session" };
      }
    }
    return allow;
  }
  if ((tool === "Agent" || tool === "Task") && input.isolation !== undefined && input.isolation !== null) {
    return { kind: "deny", reason: "Subagents work in this session's folder; worktrees and remote runs are the Orchestrator's" };
  }
  if (SESSION_TOOLS.has(tool)) {
    return allow;
  }
  if (tool.startsWith("mcp__")) {
    const named = policy.mcpServers.find((s) => tool.startsWith(`mcp__${s}__`) && !tool.slice(`mcp__${s}__`.length).includes("__"));
    return named !== undefined
      ? allow
      : { kind: "ask", reason: `${tool} belongs to an MCP server this session was not given` };
  }
  return { kind: "ask", reason: `${tool} is not covered by this session's permission policy` };
}

/** Names that look like keys, tokens or other secrets. */
const SECRET = /KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL|AUTH|COOKIE|PRIVATE/i;

/** Variables of a Claude Code session that started the Studio, never inherited. */
const PARENT_SESSION = /^(CLAUDECODE|CLAUDE_CODE_|CLAUDE_AGENT_SDK_|ANTHROPIC_|OTEL_)/;

/**
 * The agent process's environment.
 *
 * Without a policy it is built from nothing: Windows' required variables,
 * the key, the agent's own folders, and the documented switches that turn
 * off auto memory, CLAUDE.md files and nonessential traffic (ROADMAP [107]).
 *
 * With a policy it is the companion's own environment (which the Studio
 * already filtered) without anything that looks like a secret or belongs to
 * a Claude Code session that started the Studio; plus the model's key and
 * endpoint, the SDK's configuration folder and the switches that turn off
 * auto memory and nonessential traffic and keep the key out of the session's
 * commands. The home folder stays the Operator's, so Cargo, git and the
 * toolchains find their configuration.
 */
export function agentEnvironment(
  options: Pick<StartOptions, "configDir" | "home" | "policy" | "endpoint" | "model">,
  from: Record<string, string | undefined>,
  client: string,
): Record<string, string> {
  const env: Record<string, string> = {};
  if (options.policy === null) {
    for (const name of ["SystemRoot", "SYSTEMROOT", "windir", "SystemDrive", "TEMP", "TMP", "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE"]) {
      const value = from[name];
      if (value) {
        env[name] = value;
      }
    }
    const systemRoot = from.SystemRoot ?? from.SYSTEMROOT;
    if (systemRoot) {
      env.PATH = `${systemRoot}\\System32`;
    }
    env.USERPROFILE = options.home;
    env.HOME = options.home;
    env.CLAUDE_CODE_DISABLE_CLAUDE_MDS = "1";
  } else {
    for (const [name, value] of Object.entries(from)) {
      if (value !== undefined && !SECRET.test(name) && !PARENT_SESSION.test(name)) {
        env[name] = value;
      }
    }
    if (!options.policy.network) {
      env.CARGO_NET_OFFLINE = "true";
    }
    // The model's key stays out of the environment of the session's commands,
    // hooks and MCP servers (measured on 2026-10-03: without this a command
    // could read it). Only here: a session without a policy runs no command,
    // and the switch makes the SDK report the `default` permission mode.
    env.CLAUDE_CODE_SUBPROCESS_ENV_SCRUB = "1";
  }
  if (from.ANTHROPIC_API_KEY) {
    env.ANTHROPIC_API_KEY = from.ANTHROPIC_API_KEY;
  }
  if (options.endpoint !== null) {
    env.ANTHROPIC_BASE_URL = options.endpoint.baseUrl;
    const main = options.model ?? undefined;
    const fast = options.endpoint.fastModel ?? main;
    if (main) {
      env.ANTHROPIC_MODEL = main;
      env.ANTHROPIC_DEFAULT_OPUS_MODEL = main;
      env.ANTHROPIC_DEFAULT_SONNET_MODEL = main;
    }
    if (fast) {
      env.ANTHROPIC_DEFAULT_HAIKU_MODEL = fast;
      env.ANTHROPIC_SMALL_FAST_MODEL = fast;
    }
  }
  return {
    ...env,
    CLAUDE_CONFIG_DIR: options.configDir,
    CLAUDE_CODE_DISABLE_AUTO_MEMORY: "1",
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
    DISABLE_TELEMETRY: "1",
    DISABLE_ERROR_REPORTING: "1",
    CLAUDE_AGENT_SDK_CLIENT_APP: client,
  };
}

/** A pre-tool hook's answer: allow, deny or ask, with the reason. */
export function hookAnswer(decision: "allow" | "deny" | "ask", reason: string) {
  return {
    hookSpecificOutput: {
      hookEventName: "PreToolUse" as const,
      permissionDecision: decision,
      permissionDecisionReason: reason,
    },
  };
}

/** A pre-tool hook's answer: deny, with the reason the agent reads. */
export function deny(reason: string) {
  return hookAnswer("deny", reason);
}

/** What the hook does before deciding (the pause gate), and with a call to ask about. */
export interface HookHost {
  /** Waits while the session is paused; resolves when the call may go on. */
  gate(tool: string): Promise<void>;
  /** Why the turn was stopped, or null while it runs. */
  stopped(): string | null;
  /** Asks the Studio about a call the policy does not decide. */
  ask(tool: string, input: Record<string, unknown>, reason: string, signal?: AbortSignal): Promise<{ allow: boolean; message: string }>;
}

/**
 * The SDK options for one turn. `prompt`, `mcpServer` and `abort` are the
 * turn's own; everything that decides what the agent can do is fixed here.
 */
export function sdkOptions(
  start: StartOptions,
  mcpServer: unknown,
  abort: AbortController,
  env: Record<string, string>,
  onStderr: (line: string) => void,
  host: HookHost = { gate: async () => {}, stopped: () => null, ask: async () => ({ allow: false, message: "Nobody can answer." }) },
) {
  const names = start.tools.map((t) => t.name);
  const common = {
    cwd: start.cwd,
    abortController: abort,
    allowedTools: names.map(qualified),
    mcpServers: { [SERVER]: { type: "sdk" as const, name: SERVER, instance: mcpServer } },
    // The Operator's text is delivered as written: no slash commands, no
    // `@path` file expansion.
    verbatimPrompts: true,
    model: start.model ?? undefined,
    effort: start.effort ?? undefined,
    maxTurns: start.maxTurns,
    resume: start.resume ?? undefined,
    // Each turn forks the session it continues, so a session holds exactly
    // the turns up to its own: after the Operator edits or retries an earlier
    // message, the session of the turn before it is still exactly right.
    forkSession: start.resume ? true : undefined,
    persistSession: true,
    includePartialMessages: true,
    env,
    stderr: onStderr,
  };
  const policy = start.policy;
  if (policy === null) {
    return {
      ...common,
      // No built-in tool at all, and each named again as disallowed.
      tools: [] as string[],
      disallowedTools: BUILT_IN_TOOLS,
      // Exactly Agentique's tools are allowed; anything else is denied without
      // asking (`dontAsk`), and nobody is asked (`permissionPrompts: none`).
      permissionMode: "dontAsk" as const,
      permissionPrompts: "none" as const,
      canUseTool: async () => ({
        behavior: "deny" as const,
        message: "Only Agentique's own tools may be used.",
      }),
      hooks: {
        PreToolUse: [
          {
            hooks: [
              async (input: { tool_name?: string }) => {
                if (agentiqueTool(input.tool_name ?? "", names) === null) {
                  return deny(`${input.tool_name} is not one of Agentique's tools.`);
                }
                await host.gate(input.tool_name ?? "");
                const stopped = host.stopped();
                return stopped === null ? { continue: true } : deny(`Not run: ${stopped}.`);
              },
            ],
          },
        ],
      },
      strictMcpConfig: true,
      // No filesystem settings, skills or plugins of the machine's own.
      settingSources: [] as never[],
      skills: [] as string[],
      plugins: [] as never[],
      agents: {},
      systemPrompt: { type: "custom" as const, prompt: start.systemPrompt, snapshot: true },
      thinking: { type: "adaptive" as const, display: "summarized" as const },
    };
  }
  const extraRead = policy.read.filter((r) => normalise(r, start.cwd) !== normalise(start.cwd, start.cwd));
  return {
    ...common,
    tools: { type: "preset" as const, preset: "claude_code" as const },
    // Key files are also denied to the SDK's own Read rules, which its
    // search turns into exclusions.
    disallowedTools: [...DEVELOPMENT_DISALLOWED, ...policy.hidden.map((h) => `Read(${h})`)],
    additionalDirectories: extraRead,
    // The highest settings layer: the project's own settings cannot turn the
    // key scrub off or send the key elsewhere, and without trusted-local
    // execution their hooks (commands) do not run.
    settings: {
      env: {
        CLAUDE_CODE_SUBPROCESS_ENV_SCRUB: "1",
        ...(start.endpoint !== null ? { ANTHROPIC_BASE_URL: start.endpoint.baseUrl } : {}),
      },
      ...(policy.commands ? {} : { disableAllHooks: true }),
    },
    // Everything the policy allows is allowed by the hook; the SDK asks
    // (through `canUseTool`) only about what the hook leaves undecided.
    permissionMode: "default" as const,
    canUseTool: async (tool: string, input: Record<string, unknown>, options: { signal?: AbortSignal; decisionReason?: string }) => {
      if (policy.undecided === "refuse") {
        return { behavior: "deny" as const, message: `${tool} is outside this session's permission policy${options.decisionReason ? ` (${options.decisionReason})` : ""}.` };
      }
      const answer = await host.ask(tool, input, options.decisionReason ?? "outside the permission policy", options.signal);
      return answer.allow
        ? { behavior: "allow" as const, updatedInput: input }
        : { behavior: "deny" as const, message: answer.message || `The Operator did not allow ${tool}.` };
    },
    hooks: {
      PreToolUse: [
        {
          // Pause holds a call inside this hook: a day, not the default.
          timeout: 86_400,
          hooks: [
            async (input: { tool_name?: string; tool_input?: unknown }) => {
              const tool = input.tool_name ?? "";
              const toolInput = (typeof input.tool_input === "object" && input.tool_input !== null ? input.tool_input : {}) as Record<string, unknown>;
              const decision = decide(tool, toolInput, policy, start.cwd, names);
              if (decision.kind === "deny") {
                return deny(decision.reason);
              }
              await host.gate(tool);
              // Stopped while it was held: the call does not run.
              const stopped = host.stopped();
              if (stopped !== null) {
                return deny(`Not run: ${stopped}.`);
              }
              if (decision.kind === "allow") {
                return hookAnswer("allow", "allowed by the session's permission policy");
              }
              return policy.undecided === "ask" ? hookAnswer("ask", decision.reason) : deny(`${decision.reason}.`);
            },
          ],
        },
      ],
    },
    // The project's servers load only when the Operator gave the session some.
    strictMcpConfig: policy.mcpServers.length === 0,
    settingSources: start.settingSources,
    skills: start.settingSources.includes("project") ? ("all" as const) : ([] as string[]),
    plugins: [] as never[],
    agents: start.agents,
    systemPrompt: start.preset
      ? { type: "preset" as const, preset: "claude_code" as const, append: start.systemPrompt, snapshot: true }
      : { type: "custom" as const, prompt: start.systemPrompt, snapshot: true },
    // An Anthropic-compatible endpoint gets the model's own thinking; adaptive
    // thinking is Anthropic's.
    thinking: start.endpoint === null ? { type: "adaptive" as const, display: "summarized" as const } : undefined,
  };
}
