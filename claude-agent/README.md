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
  `result`, `error` and `log`. C-54 adds optional fields only: the
  credential the SDK uses in `init` (`apiKeySource`, `apiProvider`,
  `tokenSource`), the `source` of an `auth` error when the SDK would have
  used another credential, the error kind `limit` (a Claude plan's usage
  limit), and a session's spend ceiling in `start` (`maxBudgetUsd`). The
  Rust side is `crates/assistant/src/claude_agent.rs` and
  `crates/assistant/src/policy.rs`.
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
- **Model access and credentials** (C-54): a session is given exactly one
  credential: an API key in `ANTHROPIC_API_KEY` (Anthropic's, from Settings
  › Providers › Anthropic or the environment; or an Anthropic-compatible
  endpoint's provider's, such as DeepSeek's
  `https://api.deepseek.com/anthropic` with `deepseek-v4-pro` and
  `deepseek-flash` for the SDK's small tasks), or the Operator's own Claude
  subscription token from `claude setup-token` in `CLAUDE_CODE_OAUTH_TOKEN`
  (Anthropic's API only, with no key beside it, since a key would take
  precedence). Before the first prompt is given to the SDK, the companion
  reads which credential the SDK uses (`initializationResult().account`):
  for a key its `apiKeySource` must be `ANTHROPIC_API_KEY` and it may name
  no token; for the token its `tokenSource` must be
  `CLAUDE_CODE_OAUTH_TOKEN` on `firstParty` and it may name no key. Anything
  else (`none` from the machine's claude.ai login, an `apiKeyHelper` or
  `ANTHROPIC_AUTH_TOKEN` from a project's settings) ends the session with an
  `auth` error naming the source, and nothing reaches the model; the init
  message is checked again (`policy.ts`, `credentialProblem` and
  `initProblem`). The flag tier of the settings blanks every credential the
  session was not given and the credential helpers (`credentialSettings`),
  and a project's settings that bring a credential of their own keep the
  session from starting (`projectCredentialProblem`, by the credential and
  endpoint variables Claude Code reads, named). That check runs when a
  session starts, so a key replaced under `ANTHROPIC_API_KEY` in a
  project's settings while a key session runs is caught at the next start
  (the one case the SDK's report cannot show, `docs/stages.md`, W12.3);
  meanwhile the flag tier still turns off the helpers and every other
  credential. The SDK keeps its
  own configuration folder, so the machine's own `/login` is never read, and
  `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` keeps the credential out of the
  session's commands (measured live for the token on 2026-10-04). A refused
  key or token ends the turn at once; a Claude plan's usage limit ends it
  with the reason (kind `limit`) and never moves to a key. Subagents stay on
  the session's endpoint and credential and, unless they name a model, run
  on the session's (`CLAUDE_CODE_SUBAGENT_MODEL`, pinned in the flag tier);
  on an Anthropic-compatible endpoint every model alias (`opus`, `sonnet`,
  `haiku`) names its models; their usage is reported by model. Agentique costs sessions from their
  usage at the model's own price (on the subscription, what the API would
  have charged); the SDK's own estimate assumes Claude's prices, so a
  session's spend ceiling (`maxBudgetUsd`) is given only on Anthropic's own
  API.
- **Whether this computer has a Claude login** (`main.ts --login`): the
  SDK's Claude Code binary (or a `claude` on the PATH without it) runs the
  documented `claude auth status` with the Operator's own home folder and
  no configuration folder, key or token of Agentique's; only `loggedIn`,
  `authMethod`, `apiProvider` and `subscriptionType` are kept (never a
  token, email or organisation). Agentique never uses that login; Settings
  say why.

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
node --experimental-strip-types src/main.ts --login    # whether this computer has a Claude login
```

The Rust tests in `crates/assistant/tests/claude_agent.rs` run the protocol
against a scripted stand-in, and the real SDK with a refused key (no cost);
the live tests need `AGQ_LIVE=1` and a key, and cost a few cents:
`live_the_sdk_reads_the_model_through_agentique_and_resumes_its_session`
(Anthropic),
`live_a_development_session_on_deepseek_works_in_the_repository_within_its_policy`
(DeepSeek's endpoint) and
`live_sessions_run_only_on_the_credential_they_are_given` (DeepSeek's key
and the Claude subscription token, each when it is there).
