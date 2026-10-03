// What the agent may do, decided here and nowhere else in the companion
// (ROADMAP §4.7). The SDK's own tools are all off; the agent sees only
// Agentique's tools, served by the companion's MCP server, whose calls the
// Studio checks and carries out. Nothing of the machine's Claude Code
// configuration is loaded, and the agent's process gets an environment built
// from nothing. These are gates on what the agent can call; they are not an
// operating-system sandbox, and the Studio's own checks stay the authority.

import type { StartOptions } from "./protocol.ts";

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

/**
 * The agent process's environment, built from nothing: Windows' required
 * variables, the key, the agent's own folders, and the documented switches
 * that turn off auto memory, CLAUDE.md files and nonessential traffic
 * (ROADMAP [107]). Nothing else of the Studio's environment passes.
 */
export function agentEnvironment(
  options: Pick<StartOptions, "configDir" | "home">,
  from: Record<string, string | undefined>,
  client: string,
): Record<string, string> {
  const env: Record<string, string> = {};
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
  if (from.ANTHROPIC_API_KEY) {
    env.ANTHROPIC_API_KEY = from.ANTHROPIC_API_KEY;
  }
  return {
    ...env,
    USERPROFILE: options.home,
    HOME: options.home,
    CLAUDE_CONFIG_DIR: options.configDir,
    CLAUDE_CODE_DISABLE_AUTO_MEMORY: "1",
    CLAUDE_CODE_DISABLE_CLAUDE_MDS: "1",
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
    DISABLE_TELEMETRY: "1",
    DISABLE_ERROR_REPORTING: "1",
    CLAUDE_AGENT_SDK_CLIENT_APP: client,
  };
}

/** A pre-tool hook's answer: deny, with the reason the agent reads. */
export function deny(reason: string) {
  return {
    hookSpecificOutput: {
      hookEventName: "PreToolUse" as const,
      permissionDecision: "deny" as const,
      permissionDecisionReason: reason,
    },
  };
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
) {
  const names = start.tools.map((t) => t.name);
  return {
    cwd: start.cwd,
    abortController: abort,
    // No built-in tool at all, and each named again as disallowed.
    tools: [] as string[],
    disallowedTools: BUILT_IN_TOOLS,
    // Exactly Agentique's tools are allowed; anything else is denied without
    // asking (`dontAsk`), and nobody is asked (`permissionPrompts: none`).
    allowedTools: names.map(qualified),
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
            async (input: { tool_name?: string }) =>
              agentiqueTool(input.tool_name ?? "", names) === null
                ? deny(`${input.tool_name} is not one of Agentique's tools.`)
                : { continue: true },
          ],
        },
      ],
    },
    mcpServers: { [SERVER]: { type: "sdk" as const, name: SERVER, instance: mcpServer } },
    strictMcpConfig: true,
    // No filesystem settings, skills or plugins of the machine's own.
    settingSources: [] as never[],
    skills: [] as string[],
    plugins: [] as never[],
    agents: {},
    // The Operator's text is delivered as written: no slash commands, no
    // `@path` file expansion.
    verbatimPrompts: true,
    systemPrompt: { type: "custom" as const, prompt: start.systemPrompt, snapshot: true },
    model: start.model ?? undefined,
    effort: start.effort ?? undefined,
    thinking: { type: "adaptive" as const, display: "summarized" as const },
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
}
