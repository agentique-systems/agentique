// Credentials (C-54, ROADMAP §4.16): a session runs only on the one
// credential the Studio gave it (an API key, or the Operator's Claude
// subscription token), checked against what the SDK reports before any
// prompt reaches the model; a Claude plan's usage limit ends the session;
// whether this computer has a Claude login is read without its token, email
// or organisation. Stand-in SDKs only; no packages needed.

import assert from "node:assert/strict";
import { test } from "node:test";
import { type McpFactory, type Sdk, Turn, limitReached } from "../src/bridge.ts";
import {
  agentEnvironment,
  credentialProblem,
  initProblem,
  loginEnvironment,
  readLogin,
  sdkOptions,
} from "../src/policy.ts";
import { type CompanionMessage, type StartOptions, decode } from "../src/protocol.ts";

const KEY = { ANTHROPIC_API_KEY: "sk-test" };
const TOKEN = { CLAUDE_CODE_OAUTH_TOKEN: "oauth-test" };

const start: StartOptions = {
  prompt: "Rename the store.",
  systemPrompt: "Agentique's skills.",
  tools: [{ name: "read_model", description: "Read.", inputSchema: { type: "object" }, readOnly: true }],
  model: "claude-sonnet-5-5",
  effort: "high",
  resume: null,
  maxTurns: 10,
  cwd: "C:\\agent\\work",
  configDir: "C:\\agent\\config",
  home: "C:\\agent\\home",
  policy: null,
  settingSources: [],
  agents: {},
  endpoint: null,
  preset: false,
};

/**
 * A stand-in SDK that answers its start with `account`, then, like the real
 * one, emits nothing of a turn until it is given a prompt; `prompts` counts
 * what it was given.
 */
function standIn(account: Record<string, unknown>, turn: (prompt: unknown) => Record<string, unknown>[]) {
  const mcp: McpFactory = () => ({ stand: "in" });
  const seen = { prompts: 0 };
  const sdk: Sdk = {
    query({ prompt }) {
      const input = prompt[Symbol.asyncIterator]();
      const generator = (async function* () {
        for (;;) {
          const next = await input.next();
          if (next.done) {
            return;
          }
          seen.prompts += 1;
          yield* turn(next.value);
        }
      })();
      return Object.assign(generator, {
        initializationResult: async () => {
          await new Promise((resolve) => setTimeout(resolve, 5));
          return { account };
        },
        interrupt: async () => void (await generator.return(undefined)),
      });
    },
  };
  return { sdk, mcp, seen };
}

const answered = (apiKeySource: string) => () => [
  { type: "system", subtype: "init", session_id: "s1", model: "claude-sonnet-5-5", apiKeySource },
  { type: "result", subtype: "success", is_error: false, session_id: "s1" },
];

test("a session the SDK would run on a claude.ai login stops before any prompt is sent", async () => {
  const sent: CompanionMessage[] = [];
  // Given a key, the SDK reports no API key: the machine's own login.
  const { sdk, mcp, seen } = standIn({ apiKeySource: "none", apiProvider: "firstParty" }, answered("none"));
  await new Turn((m) => sent.push(m), sdk, mcp, KEY, "agentique/test").run(start);
  assert.equal(seen.prompts, 0, "no prompt reached the model");
  assert.deepEqual(sent.map((m) => m.type), ["error", "done"]);
  const error = sent[0];
  assert.equal(error.type === "error" && error.kind, "auth");
  assert.equal(error.type === "error" && error.source, "none");
  assert.match(error.type === "error" ? error.message : "", /claude\.ai login.*stopped before anything reached the model/);
});

test("a given key runs only when the SDK reports that key", async () => {
  const sent: CompanionMessage[] = [];
  const { sdk, mcp, seen } = standIn({ apiKeySource: "ANTHROPIC_API_KEY", apiProvider: "firstParty" }, answered("ANTHROPIC_API_KEY"));
  await new Turn((m) => sent.push(m), sdk, mcp, KEY, "agentique/test").run(start);
  assert.equal(seen.prompts, 1);
  assert.deepEqual(sent.map((m) => m.type), ["init", "result", "done"]);
  const init = sent[0];
  assert.equal(init.type === "init" && init.apiKeySource, "ANTHROPIC_API_KEY");
  assert.equal(init.type === "init" && init.apiProvider, "firstParty");
  // An apiKeyHelper from a project's settings is another credential.
  assert.match(credentialProblem({ apiKeySource: "apiKeyHelper" }, KEY) ?? "", /apiKeyHelper/);
  assert.match(credentialProblem({}, KEY) ?? "", /did not name/);
});

test("the subscription token runs only as that token on Anthropic's own API", async () => {
  const sent: CompanionMessage[] = [];
  const { sdk, mcp, seen } = standIn({ apiProvider: "firstParty", tokenSource: "CLAUDE_CODE_OAUTH_TOKEN" }, answered("none"));
  await new Turn((m) => sent.push(m), sdk, mcp, TOKEN, "agentique/test").run(start);
  assert.equal(seen.prompts, 1);
  assert.deepEqual(sent.map((m) => m.type), ["init", "result", "done"]);
  const init = sent[0];
  assert.equal(init.type === "init" && init.tokenSource, "CLAUDE_CODE_OAUTH_TOKEN");
  // With the token the init message says no API key, as it should.
  assert.equal(init.type === "init" && init.apiKeySource, "none");
  // The machine's own claude.ai login, another provider, two credentials
  // or none are refused.
  assert.ok(credentialProblem({ apiProvider: "firstParty", tokenSource: "claude.ai" }, TOKEN));
  assert.ok(credentialProblem({ apiProvider: "bedrock", tokenSource: "CLAUDE_CODE_OAUTH_TOKEN" }, TOKEN));
  assert.match(credentialProblem({ apiKeySource: "ANTHROPIC_API_KEY" }, { ...KEY, ...TOKEN }) ?? "", /two credentials/);
  assert.match(credentialProblem({ apiKeySource: "ANTHROPIC_API_KEY" }, {}) ?? "", /no credential/);
  // The init message is checked again: an API key with the token is wrong.
  assert.equal(initProblem("none", TOKEN), null);
  assert.ok(initProblem("ANTHROPIC_API_KEY", TOKEN));
  assert.ok(initProblem("none", KEY));
  assert.equal(initProblem(undefined, KEY), null);
});

test("a token the SDK does not report as given stops the session unprompted", async () => {
  const sent: CompanionMessage[] = [];
  const { sdk, mcp, seen } = standIn({ apiProvider: "firstParty" }, answered("none"));
  await new Turn((m) => sent.push(m), sdk, mcp, TOKEN, "agentique/test").run(start);
  assert.equal(seen.prompts, 0);
  const error = sent.find((m) => m.type === "error");
  assert.equal(error?.type === "error" && error.kind, "auth");
});

test("the session gets exactly one credential: with the token, no API key", () => {
  for (const policy of [null, { read: [], write: [], protected: [], hidden: [], commands: false, refusedCommands: [], network: false, mcpServers: [], undecided: "refuse" as const }]) {
    const options = { ...start, policy };
    const token = agentEnvironment(options, { ...TOKEN, SystemRoot: "C:\\Windows", PATH: "C:\\tools" }, "agentique/test");
    assert.equal(token.CLAUDE_CODE_OAUTH_TOKEN, "oauth-test");
    assert.equal(token.ANTHROPIC_API_KEY, undefined);
    const key = agentEnvironment(options, { ...KEY, SystemRoot: "C:\\Windows" }, "agentique/test");
    assert.equal(key.ANTHROPIC_API_KEY, "sk-test");
    assert.equal(key.CLAUDE_CODE_OAUTH_TOKEN, undefined);
  }
  // In a development session the token is scrubbed from the session's
  // commands like a key.
  const development = agentEnvironment({ ...start, policy: { read: [], write: [], protected: [], hidden: [], commands: true, refusedCommands: [], network: false, mcpServers: [], undecided: "refuse" } }, TOKEN, "agentique/test");
  assert.equal(development.CLAUDE_CODE_SUBPROCESS_ENV_SCRUB, "1");
});

test("a Claude plan's usage limit ends the session with the reason", async () => {
  const sent: CompanionMessage[] = [];
  const { sdk, mcp } = standIn({ apiProvider: "firstParty", tokenSource: "CLAUDE_CODE_OAUTH_TOKEN" }, () => [
    { type: "system", subtype: "init", session_id: "s1", apiKeySource: "none" },
    { type: "rate_limit_event", rate_limit_info: { status: "rejected", rateLimitType: "five_hour", resetsAt: 1_791_000_000 } },
    { type: "result", subtype: "success", is_error: false, session_id: "s1" },
  ]);
  await new Turn((m) => sent.push(m), sdk, mcp, TOKEN, "agentique/test").run(start);
  const error = sent.find((m) => m.type === "error");
  assert.equal(error?.type === "error" && error.kind, "limit");
  assert.match(error?.type === "error" ? error.message : "", /usage limit is reached \(five hour\); it resets at .* UTC\. .*does not move to an API key/);
  assert.match(limitReached({}), /^The Claude plan's usage limit is reached\./);
});

test("the spend ceiling reaches the SDK, and only as a positive amount", () => {
  const options = sdkOptions({ ...start, maxBudgetUsd: 1.25 }, {}, new AbortController(), {}, () => {});
  assert.equal(options.maxBudgetUsd, 1.25);
  assert.equal(sdkOptions(start, {}, new AbortController(), {}, () => {}).maxBudgetUsd, undefined);
  const line = (maxBudgetUsd: unknown) => JSON.stringify({ type: "start", options: { ...start, maxBudgetUsd } });
  assert.equal(decode(line(2)).type, "start");
  assert.equal(decode(line(null)).type, "start");
  assert.throws(() => decode(line(-1)), /maxBudgetUsd/);
  assert.throws(() => decode(line("5")), /maxBudgetUsd/);
});

test("a local Claude login is read as whether and how, never who", () => {
  // The shape `claude auth status` prints (values made up).
  const status = JSON.stringify({
    loggedIn: true,
    authMethod: "claude.ai",
    apiProvider: "firstParty",
    analyticsDisabled: false,
    projectsDirectory: "C:\\Users\\someone\\.claude\\projects",
    configDirectory: "C:\\Users\\someone\\.claude",
    email: "someone@example.invalid",
    orgId: "org-1",
    orgName: "Someone's Organization",
    subscriptionType: "max",
  });
  assert.deepEqual(readLogin(status), { loggedIn: true, authMethod: "claude.ai", apiProvider: "firstParty", subscriptionType: "max" });
  assert.deepEqual(readLogin('{"loggedIn": false, "authMethod": "none"}\n'), { loggedIn: false, authMethod: "none", apiProvider: null, subscriptionType: null });
  assert.equal(readLogin("Not logged in"), null);
  assert.equal(readLogin('{"authMethod": "claude.ai"}'), null);
  // The probe reads the Operator's own configuration: no configuration
  // folder of Agentique's, no key or token, no Claude Code session.
  const env = loginEnvironment({
    USERPROFILE: "C:\\Users\\someone",
    APPDATA: "C:\\Users\\someone\\AppData\\Roaming",
    PATH: "C:\\tools",
    CLAUDE_CONFIG_DIR: "C:\\agq\\config",
    ANTHROPIC_API_KEY: "sk-x",
    ANTHROPIC_BASE_URL: "https://api.deepseek.com/anthropic",
    CLAUDE_CODE_OAUTH_TOKEN: "oauth-x",
    CLAUDECODE: "1",
    GITHUB_TOKEN: "t",
  });
  assert.deepEqual(Object.keys(env).sort(), ["APPDATA", "PATH", "USERPROFILE"]);
});
