# claude-agent

The Claude Agent runtime's companion (ROADMAP §4.7, C-51; part
`ClaudeAgentRuntime` in `model/Agentique.sysml`): a small TypeScript program
that runs the official Claude Agent SDK (`@anthropic-ai/claude-agent-sdk`) for
the Assistant when Settings › Assistant › Runtime is "Claude Agent". The SDK
owns its loop; Agentique owns the project, the tools, permissions, approvals,
model changes, execution, review and adoption.

- **Protocol 1**: one JSON object per line over standard input and output,
  one companion process per turn (`src/protocol.ts`). The Studio sends
  `start`, `tool_result`, `interrupt` and `close`; the companion sends
  `ready`, `init`, `text`, `thinking`, `assistant`, `tool_call`, `retry`,
  `result`, `error` and `log`. The Rust side is
  `crates/assistant/src/claude_agent.rs`.
- **Tools**: only Agentique's, served by the companion's own MCP server
  (`agentique`) with the Studio's JSON schemas. Each call goes to the Studio,
  which checks it against its schema and carries it out (model changes as
  System State operations). The companion never acts on a call itself.
- **Policy** (`src/policy.ts`, the one place): the SDK's own tools off
  (`tools: []`) and named as disallowed; `permissionMode: "dontAsk"` with
  only Agentique's tools allowed, and a `PreToolUse` hook that denies
  anything else; no settings, `CLAUDE.md`, skills, plugins, memory or other
  MCP servers loaded from the machine (`settingSources: []`,
  `strictMcpConfig`); an environment built from nothing (the key, the
  runtime's own config folder, and switches that turn off telemetry and
  auto-memory); prompts taken verbatim. The companion checks at start that
  the SDK reports exactly this and stops otherwise. These gate what the agent
  can call; they are not an operating-system sandbox.
- **Authentication**: an Anthropic API key only (Settings › Providers ›
  Anthropic, or `ANTHROPIC_API_KEY`); claude.ai login is not offered. A
  refused key ends the turn at once instead of retrying.

The packages are pinned exactly in `package-lock.json`, including the SDK's
Claude Code binary. The Studio installs them itself on the Operator's request
(Settings › Assistant, "Install…": `npm ci --omit=dev --ignore-scripts` into
`%LOCALAPPDATA%\Agentique\runtime\`) and checks the installed version; the
companion's sources are built into the Studio and written beside them.

```sh
npm ci                     # for development: the packages and the type checker
npm test                   # the bridge's tests (Node 22.6 or later; no packages needed)
npm run typecheck
node --experimental-strip-types src/main.ts --verify   # the installed SDK's version and binary
```

The Rust tests in `crates/assistant/tests/claude_agent.rs` run the protocol
against a scripted stand-in, and the real SDK with a refused key (no cost);
`live_the_sdk_reads_the_model_through_agentique_and_resumes_its_session`
needs `AGQ_LIVE=1` and a key, and costs a few cents.
