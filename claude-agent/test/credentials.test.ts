// Credentials (C-54, ROADMAP §4.16): a session runs only on the one
// credential the Studio gave it (an API key, or the Operator's Claude
// subscription token), checked against what the SDK reports before any
// prompt reaches the model; a Claude plan's usage limit ends the session;
// whether this computer has a Claude login is read without its token, email
// or organisation. Stand-in SDKs only; no packages needed.

import assert from "node:assert/strict";
import { test } from "node:test";
import { type McpFactory, type Sdk, Turn, limitReached } from "../src/bridge.ts";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  agentEnvironment,
  credentialProblem,
  credentialSettings,
  initProblem,
  loginEnvironment,
  projectCredentialProblem,
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
function standIn(account: Record<string, unknown> | Error, turn: (prompt: unknown) => Record<string, unknown>[]) {
  const mcp: McpFactory = () => ({ stand: "in" });
  const seen = { prompts: 0, queries: 0 };
  const sdk: Sdk = {
    query({ prompt }) {
      seen.queries += 1;
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
          if (account instanceof Error) {
            throw account;
          }
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

/**
 * The review of W12.3: the bundled Claude Code reports the token whenever
 * its variable is set, yet uses a key or a helper instead when one is in
 * effect (measured 2026-10-04): both sides of the report count.
 */
test("a second credential beside the given one stops the session unprompted", async () => {
  const cases: [Record<string, string>, Record<string, unknown>, string][] = [
    // A project's env.ANTHROPIC_API_KEY, or its apiKeyHelper, beside the token.
    [TOKEN, { apiKeySource: "ANTHROPIC_API_KEY", tokenSource: "CLAUDE_CODE_OAUTH_TOKEN", apiProvider: "firstParty" }, "ANTHROPIC_API_KEY"],
    [TOKEN, { apiKeySource: "apiKeyHelper", tokenSource: "CLAUDE_CODE_OAUTH_TOKEN", apiProvider: "firstParty" }, "apiKeyHelper"],
    // A project's ANTHROPIC_AUTH_TOKEN, CLAUDE_CODE_OAUTH_TOKEN or helper beside the key.
    [KEY, { apiKeySource: "ANTHROPIC_API_KEY", tokenSource: "ANTHROPIC_AUTH_TOKEN", apiProvider: "firstParty" }, "ANTHROPIC_AUTH_TOKEN"],
    [KEY, { apiKeySource: "ANTHROPIC_API_KEY", tokenSource: "CLAUDE_CODE_OAUTH_TOKEN", apiProvider: "firstParty" }, "CLAUDE_CODE_OAUTH_TOKEN"],
    [KEY, { apiKeySource: "ANTHROPIC_API_KEY", tokenSource: "apiKeyHelper", apiProvider: "firstParty" }, "apiKeyHelper"],
  ];
  for (const [credential, account, source] of cases) {
    const sent: CompanionMessage[] = [];
    const { sdk, mcp, seen } = standIn(account, answered("none"));
    await new Turn((m) => sent.push(m), sdk, mcp, credential, "agentique/test").run(start);
    assert.equal(seen.prompts, 0, JSON.stringify(account));
    const error = sent.find((m) => m.type === "error");
    assert.equal(error?.type === "error" && error.kind, "auth", JSON.stringify(account));
    assert.equal(error?.type === "error" && error.source, source);
  }
  // What the real SDK reports for the given credential alone passes.
  assert.equal(credentialProblem({ apiKeySource: "ANTHROPIC_API_KEY", tokenSource: "none", apiProvider: "firstParty" }, KEY), null);
  assert.equal(credentialProblem({ tokenSource: "CLAUDE_CODE_OAUTH_TOKEN", apiProvider: "firstParty" }, TOKEN), null);
  assert.equal(credentialProblem({ apiKeySource: "none", tokenSource: "CLAUDE_CODE_OAUTH_TOKEN", apiProvider: "firstParty" }, TOKEN), null);
});

test("the flag tier turns off every credential the session was not given", () => {
  const token = credentialSettings(TOKEN);
  assert.deepEqual(token.env, {
    ANTHROPIC_AUTH_TOKEN: "",
    CLAUDE_CODE_USE_BEDROCK: "",
    CLAUDE_CODE_USE_VERTEX: "",
    CLAUDE_CODE_USE_FOUNDRY: "",
    ANTHROPIC_API_KEY: "",
  });
  const key = credentialSettings(KEY);
  assert.equal(key.env.CLAUDE_CODE_OAUTH_TOKEN, "");
  assert.equal(key.env.ANTHROPIC_API_KEY, undefined, "the given key is never blanked or written here");
  for (const settings of [token, key]) {
    assert.equal(settings.apiKeyHelper, "");
    assert.equal(settings.awsCredentialExport, "");
  }
  // Both kinds of session carry them; a development session keeps its own
  // pins beside them.
  const plain = sdkOptions(start, {}, new AbortController(), agentEnvironment(start, TOKEN, "t"), () => {});
  assert.deepEqual((plain as unknown as { settings: { env: Record<string, string> } }).settings.env, token.env);
  const policy = { read: [], write: [], protected: [], hidden: [], commands: false, refusedCommands: [], network: false, mcpServers: [], undecided: "refuse" as const };
  const development = { ...start, policy };
  const options = sdkOptions(development, {}, new AbortController(), agentEnvironment(development, KEY, "t"), () => {}) as unknown as {
    settings: { env: Record<string, string>; apiKeyHelper: string };
  };
  assert.equal(options.settings.apiKeyHelper, "");
  assert.equal(options.settings.env.CLAUDE_CODE_OAUTH_TOKEN, "");
  assert.equal(options.settings.env.ANTHROPIC_AUTH_TOKEN, "");
  assert.equal(options.settings.env.CLAUDE_CODE_SUBPROCESS_ENV_SCRUB, "1");
  assert.equal(options.settings.env.ANTHROPIC_API_KEY, undefined);
});

test("a project's settings that bring a credential keep the session from starting", async () => {
  const project = (files: Record<string, string>) => {
    const dir = mkdtempSync(join(tmpdir(), "agq-project-"));
    mkdirSync(join(dir, ".claude"));
    for (const [name, text] of Object.entries(files)) {
      writeFileSync(join(dir, ".claude", name), text);
    }
    return dir;
  };
  const replaced = project({ "settings.json": JSON.stringify({ env: { ANTHROPIC_API_KEY: "sk-other", RUST_LOG: "info" } }) });
  assert.match(projectCredentialProblem(replaced, ["project"]) ?? "", /env[.]ANTHROPIC_API_KEY/);
  assert.match(projectCredentialProblem(project({ "settings.json": JSON.stringify({ apiKeyHelper: "echo k" }) }), ["project"]) ?? "", /apiKeyHelper/);
  assert.match(projectCredentialProblem(project({ "settings.json": JSON.stringify({ env: { CLAUDE_CODE_USE_BEDROCK: "1" } }) }), ["project"]) ?? "", /CLAUDE_CODE_USE_BEDROCK/);
  assert.match(projectCredentialProblem(project({ "settings.json": "{ not json" }), ["project"]) ?? "", /cannot be read as JSON/);
  // Ordinary settings, settings not loaded, and no settings start.
  assert.equal(projectCredentialProblem(project({ "settings.json": JSON.stringify({ env: { RUST_LOG: "info" }, permissions: { allow: [] } }) }), ["project"]), null);
  const local = project({ "settings.local.json": JSON.stringify({ env: { ANTHROPIC_AUTH_TOKEN: "x" } }) });
  assert.equal(projectCredentialProblem(local, ["project"]), null);
  assert.match(projectCredentialProblem(local, ["project", "local"]) ?? "", /ANTHROPIC_AUTH_TOKEN/);
  assert.equal(projectCredentialProblem(replaced, []), null);
  // The turn refuses before the SDK is even started.
  const sent: CompanionMessage[] = [];
  const { sdk, mcp, seen } = standIn({ apiKeySource: "ANTHROPIC_API_KEY", apiProvider: "firstParty" }, answered("ANTHROPIC_API_KEY"));
  await new Turn((m) => sent.push(m), sdk, mcp, KEY, "agentique/test").run({ ...start, cwd: replaced, settingSources: ["project"] });
  assert.equal(seen.queries, 0, "the SDK never started");
  assert.deepEqual(sent.map((m) => m.type), ["error", "done"]);
  const error = sent[0];
  assert.equal(error.type === "error" && error.kind, "auth");
  assert.equal(error.type === "error" && error.source, "project settings");
});

test("an SDK that does not start is a runtime failure with its cause, and nothing is sent", async () => {
  const sent: CompanionMessage[] = [];
  const { sdk, mcp, seen } = standIn(new Error("the Claude Code binary exited with code 3"), answered("ANTHROPIC_API_KEY"));
  await new Turn((m) => sent.push(m), sdk, mcp, KEY, "agentique/test").run(start);
  assert.equal(seen.prompts, 0);
  const error = sent.find((m) => m.type === "error");
  assert.equal(error?.type === "error" && error.kind, "runtime");
  assert.match(error?.type === "error" ? error.message : "", /did not start.*exited with code 3/);
});

test("a session's subagents run on its own model unless they name one", () => {
  const env = agentEnvironment(start, KEY, "agentique/test");
  assert.equal(env.CLAUDE_CODE_SUBAGENT_MODEL, "claude-sonnet-5-5");
  assert.equal(agentEnvironment({ ...start, model: null }, KEY, "t").CLAUDE_CODE_SUBAGENT_MODEL, undefined);
  // A development session pins it above the project's settings.
  const policy = { read: [], write: [], protected: [], hidden: [], commands: false, refusedCommands: [], network: false, mcpServers: [], undecided: "refuse" as const };
  const development = { ...start, policy };
  const options = sdkOptions(development, {}, new AbortController(), agentEnvironment(development, KEY, "t"), () => {}) as unknown as {
    settings: { env: Record<string, string> };
  };
  assert.equal(options.settings.env.CLAUDE_CODE_SUBAGENT_MODEL, "claude-sonnet-5-5");
});
