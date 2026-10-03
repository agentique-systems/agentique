# claude-agent

The Claude Agent runtime's companion (ROADMAP §4.7, C-51, C-53; part
`ClaudeAgentRuntime` in `model/Agentique.sysml`): a small TypeScript program
that runs the official Claude Agent SDK (`@anthropic-ai/claude-agent-sdk`) for
the Assistant when Settings › Assistant › Runtime is "Claude Agent", and for
the Orchestrator's agent sessions. The SDK owns its loop; Agentique owns the
project, the permission policy, Agentique's tools, model changes, execution,
review and adoption.

- **Protocol 2**: one JSON object per line over standard input and output,
  one companion process per turn (`src/protocol.ts`). The Studio sends
  `start`, `tool_result`, `permission_result`, `message` (queued into the
  running session), `gate` (`run`, `pause`, `step`), `interrupt` and
  `close`; the companion sends `ready`, `init`, `text`, `thinking`,
  `assistant`, `tool_call`, `tool_done` (one of the SDK's own tools
  finished), `permission` (a call the policy leaves undecided), `task`
  (subagents and background commands), `compaction`, `paused`, `retry`,
  `result`, `error` and `log`. The Rust side is
  `crates/assistant/src/claude_agent.rs` and `crates/assistant/src/policy.rs`.
- **Two kinds of session** (`src/policy.ts`, the one place):
  - *Without a policy*: the SDK's own tools are all off; the agent calls only
    Agentique's tools (`permissionMode: "dontAsk"`, a pre-tool hook that
    denies anything else); nothing of the machine's configuration is loaded
    (`settingSources: []`, `strictMcpConfig`); an environment built from
    nothing. The companion checks at start that the SDK reports exactly this.
  - *A development session* (C-53): the SDK's own tools (files, commands,
    subagents, skills, web fetch, task lists) under the **permission policy**
    the Studio sends. One pre-tool hook (`decide`) allows what the policy
    allows, so normal work never waits; refuses what it forbids, with the
    reason the agent reads (a model file is always refused: the model
    changes only through Agentique's tools); and asks the Studio about
    anything else (or refuses it, in an objective's sessions). The project's
    settings, `CLAUDE.md` (which imports `AGENTS.md`), skills and subagents
    are loaded (`settingSources: ["project"]`), never the machine's user
    settings or auto memory. The environment is the Studio's minus anything
    that looks like a key, token or secret; the SDK keeps the model's key out
    of the session's commands, hooks and MCP servers
    (`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`). Agentique's tools are served beside
    the SDK's by the companion's MCP server (`agentique`); the Studio checks
    and carries out every call.
  These gate what the agent can do; they are not an operating-system
  sandbox. File tools are held to the policy's folders; commands are not
  confined, so the policy refuses the known dangerous forms and the
  Orchestrator checks a change path by path before merging it.
- **Model access**: an Anthropic API key (Settings › Providers › Anthropic,
  or `ANTHROPIC_API_KEY`), or an Anthropic-compatible endpoint with its
  provider's key (DeepSeek: `https://api.deepseek.com/anthropic`,
  `deepseek-v4-pro` with `deepseek-flash` for the SDK's small tasks).
  claude.ai login is not offered. A refused key ends the turn at once.
  Agentique costs sessions from their usage at the model's own price; the
  SDK's own estimate assumes Claude's prices.

The packages are pinned exactly in `package-lock.json`, including the SDK's
Claude Code binary. The Studio installs them itself on the Operator's request
(Settings › Assistant, "Install…": `npm ci --omit=dev --ignore-scripts` into
`%LOCALAPPDATA%\Agentique\runtime\`) and checks the installed version; the
companion's sources are built into the Studio and written beside them.

```sh
npm ci                     # for development: the packages and the type checker
npm test                   # the companion's tests (Node 22.6 or later; no packages needed)
npm run typecheck
node --experimental-strip-types src/main.ts --verify   # the installed SDK's version and binary
```

The Rust tests in `crates/assistant/tests/claude_agent.rs` run the protocol
against a scripted stand-in, and the real SDK with a refused key (no cost);
the live tests need `AGQ_LIVE=1` and a key, and cost a few cents:
`live_the_sdk_reads_the_model_through_agentique_and_resumes_its_session`
(Anthropic) and
`live_a_development_session_on_deepseek_works_in_the_repository_within_its_policy`
(DeepSeek's endpoint).
