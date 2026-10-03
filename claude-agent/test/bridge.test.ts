// The companion's own tests (ROADMAP §4.7, W10.3): the protocol, the
// permission configuration and one turn against a stand-in SDK. They need no
// installed packages. The real SDK is exercised by `main.ts --probe` (the
// Studio's health check) and by the Studio's integration tests.

import assert from "node:assert/strict";
import { test } from "node:test";
import { type McpFactory, type Sdk, Turn, canonical } from "../src/bridge.ts";
import {
  BUILT_IN_TOOLS,
  agentEnvironment,
  agentiqueTool,
  qualified,
  sdkOptions,
} from "../src/policy.ts";
import { type CompanionMessage, LineSplitter, type StartOptions, decode, encode } from "../src/protocol.ts";

const start: StartOptions = {
  prompt: "Rename the store.",
  systemPrompt: "You are Agentique's Assistant.",
  tools: [
    { name: "read_model", description: "Read.", inputSchema: { type: "object" }, readOnly: true },
    { name: "apply_changes", description: "Change.", inputSchema: { type: "object" } },
  ],
  model: "claude-opus-5-5",
  effort: "high",
  resume: null,
  maxTurns: 40,
  cwd: "C:\\agent\\work",
  configDir: "C:\\agent\\config",
  home: "C:\\agent\\home",
};

test("protocol messages are decoded strictly", () => {
  assert.deepEqual(decode('{"type":"interrupt"}'), { type: "interrupt" });
  assert.deepEqual(decode('{"type":"tool_result","call":"c1","content":"ok"}'), {
    type: "tool_result",
    call: "c1",
    content: "ok",
    isError: false,
  });
  assert.throws(() => decode('{"type":"run_shell"}'), /unknown message type/);
  assert.throws(() => decode("[1]"), /JSON object/);
  assert.throws(() => decode('{"type":"start","options":{"prompt":"x","tools":[]}}'), /options.cwd/);
  const splitter = new LineSplitter();
  assert.deepEqual(splitter.push('{"a":1}\r\n{"b"'), ['{"a":1}']);
  assert.deepEqual(splitter.push(":2}\n"), ['{"b":2}']);
  assert.equal(encode({ type: "text", text: "hi" }), '{"type":"text","text":"hi"}\n');
});

test("the agent gets no built-in tool, no machine settings and only Agentique's tools", async () => {
  const env = agentEnvironment(start, { SystemRoot: "C:\\Windows", ANTHROPIC_API_KEY: "k", GITHUB_TOKEN: "t", CLAUDECODE: "1", PATH: "C:\\tools" }, "agentique/test");
  const options = sdkOptions(start, {}, new AbortController(), env, () => {});
  assert.deepEqual(options.tools, []);
  for (const tool of ["Bash", "Read", "Write", "Edit", "Skill", "Agent", "Task", "WebFetch", "WebSearch"]) {
    assert.ok(BUILT_IN_TOOLS.includes(tool), tool);
    assert.ok(options.disallowedTools.includes(tool), tool);
  }
  assert.deepEqual(options.allowedTools, ["mcp__agentique__read_model", "mcp__agentique__apply_changes"]);
  assert.equal(options.permissionMode, "dontAsk");
  assert.equal(options.permissionPrompts, "none");
  assert.deepEqual(options.settingSources, []);
  assert.equal(options.strictMcpConfig, true);
  assert.deepEqual(options.plugins, []);
  assert.equal(options.verbatimPrompts, true);
  assert.equal(options.cwd, start.cwd);
  assert.deepEqual(Object.keys(options.mcpServers), ["agentique"]);
  // The environment is built from nothing: no tokens, no parent session.
  assert.equal(env.ANTHROPIC_API_KEY, "k");
  assert.equal(env.GITHUB_TOKEN, undefined);
  assert.equal(env.CLAUDECODE, undefined);
  assert.equal(env.PATH, "C:\\Windows\\System32");
  assert.equal(env.CLAUDE_CONFIG_DIR, start.configDir);
  assert.equal(env.USERPROFILE, start.home);
  assert.equal(env.CLAUDE_CODE_DISABLE_AUTO_MEMORY, "1");
  assert.equal(env.CLAUDE_CODE_DISABLE_CLAUDE_MDS, "1");
  // The pre-tool hook denies any other tool, whatever the SDK allows.
  const hook = options.hooks.PreToolUse[0].hooks[0];
  const denied = (await hook({ tool_name: "Bash" })) as { hookSpecificOutput?: { permissionDecision: string } };
  assert.equal(denied.hookSpecificOutput?.permissionDecision, "deny");
  const other = (await hook({ tool_name: "mcp__other__read_model" })) as { hookSpecificOutput?: { permissionDecision: string } };
  assert.equal(other.hookSpecificOutput?.permissionDecision, "deny");
  assert.deepEqual(await hook({ tool_name: qualified("read_model") }), { continue: true });
  assert.equal(agentiqueTool("mcp__agentique__delete_everything", ["read_model"]), null);
  // And nothing reaches canUseTool's approval.
  assert.equal((await options.canUseTool()).behavior, "deny");
  // A new conversation starts a session; a later turn forks the one before.
  assert.equal(options.resume, undefined);
  assert.equal(options.forkSession, undefined);
  const resumed = sdkOptions({ ...start, resume: "s1" }, {}, new AbortController(), env, () => {});
  assert.equal(resumed.resume, "s1");
  assert.equal(resumed.forkSession, true);
});

/** A stand-in SDK: plays `script`, calling tools through the MCP factory. */
function standIn(script: (call: (name: string, input: Record<string, unknown>) => Promise<{ content: string; isError: boolean }>) => AsyncGenerator<Record<string, unknown>>) {
  let tools: ((name: string, input: Record<string, unknown>) => Promise<{ content: string; isError: boolean }>) | null = null;
  const mcp: McpFactory = (_definitions, call) => {
    tools = (name, input) => call(name, input);
    return { stand: "in" };
  };
  let interrupted = 0;
  const sdk: Sdk = {
    query() {
      const generator = script((name, input) => tools!(name, input));
      return Object.assign(generator, {
        interrupt: async () => {
          interrupted += 1;
          await generator.return(undefined);
        },
      });
    },
  };
  return { sdk, mcp, interrupts: () => interrupted };
}

test("a turn carries tool calls to the Studio one at a time, with their tool use ids", async () => {
  const sent: CompanionMessage[] = [];
  const { sdk, mcp } = standIn(async function* (call) {
    yield { type: "system", subtype: "init", session_id: "s1", model: "claude-opus-5-5", tools: [qualified("read_model")], mcp_servers: [{ name: "agentique", status: "connected" }], permissionMode: "dontAsk" };
    yield { type: "stream_event", event: { type: "content_block_delta", delta: { type: "text_delta", text: "Reading." } } };
    yield {
      type: "assistant",
      parent_tool_use_id: null,
      message: { model: "claude-opus-5-5", content: [
        { type: "tool_use", id: "toolu_1", name: "read_model", input: { element: "Shop" } },
        { type: "tool_use", id: "toolu_2", name: "read_model", input: { element: "Store" } },
      ] },
    };
    // The SDK may run read-only calls at the same time: the Studio still
    // gets them one by one.
    const [a, b] = await Promise.all([
      call("read_model", { element: "Store" }),
      call("read_model", { element: "Shop" }),
    ]);
    assert.equal(a.content, "the store");
    assert.equal(b.content, "the shop");
    yield { type: "result", subtype: "success", is_error: false, num_turns: 2, total_cost_usd: 0.01, session_id: "s1", stop_reason: "end_turn", usage: { input_tokens: 10, output_tokens: 5 } };
  });
  let turn: Turn;
  const answers: Record<string, string> = { Store: "the store", Shop: "the shop" };
  let inFlight = 0;
  turn = new Turn(
    (m) => {
      sent.push(m);
      if (m.type === "tool_call") {
        inFlight += 1;
        assert.equal(inFlight, 1, "one call at a time");
        setTimeout(() => {
          inFlight -= 1;
          turn.answer(m.call, { content: answers[String(m.input.element)], isError: false });
        }, 5);
      }
    },
    sdk,
    mcp,
    { SystemRoot: "C:\\Windows" },
    "agentique/test",
  );
  await turn.run(start);
  const calls = sent.filter((m) => m.type === "tool_call");
  assert.equal(calls.length, 2);
  // Each call carries the id of the tool use it answers.
  assert.deepEqual(
    calls.map((c) => (c.type === "tool_call" ? [c.toolUseId, c.input.element] : null)),
    [["toolu_2", "Store"], ["toolu_1", "Shop"]],
  );
  assert.deepEqual(sent.map((m) => m.type), ["init", "text", "assistant", "tool_call", "tool_call", "result"]);
  const result = sent.at(-1);
  assert.equal(result?.type === "result" && result.sessionId, "s1");
});

test("a stopped turn answers its waiting call not run and is interrupted", async () => {
  const sent: CompanionMessage[] = [];
  let answer: { content: string; isError: boolean } | null = null;
  const { sdk, mcp, interrupts } = standIn(async function* (call) {
    yield { type: "assistant", parent_tool_use_id: null, message: { content: [{ type: "tool_use", id: "toolu_9", name: "apply_changes", input: {} }] } };
    answer = await call("apply_changes", {});
    yield { type: "result", subtype: "error_during_execution", is_error: true, session_id: "s2" };
  });
  const turn: Turn = new Turn(
    (m) => {
      sent.push(m);
      if (m.type === "tool_call") {
        // The Operator stops while the change waits for the lock question.
        void turn.stop("stopped by the Operator");
      }
    },
    sdk,
    mcp,
    {},
    "agentique/test",
  );
  await turn.run(start);
  assert.deepEqual(answer, { content: "Not run: stopped by the Operator.", isError: true });
  assert.equal(interrupts(), 1);
  const error = sent.find((m) => m.type === "error");
  assert.deepEqual(error, { type: "error", kind: "interrupted", message: "stopped by the Operator" });
  // An answer that comes too late changes nothing.
  turn.answer("c1", { content: "applied", isError: false });
});

test("a refused key ends the turn at once with a plain reason", async () => {
  const sent: CompanionMessage[] = [];
  const { sdk, mcp } = standIn(async function* () {
    yield { type: "system", subtype: "init", session_id: "s3" };
    yield { type: "system", subtype: "api_retry", attempt: 1, error: "authentication_failed", error_status: 401 };
    throw new Error("must not be reached: the turn stops at the refused key");
  });
  await new Turn((m) => sent.push(m), sdk, mcp, {}, "agentique/test").run(start);
  assert.deepEqual(sent.map((m) => m.type), ["init", "retry", "error"]);
  const error = sent.at(-1);
  assert.equal(error?.type === "error" && error.kind, "auth");
});

test("a tool the Studio did not define is never sent to it", async () => {
  const sent: CompanionMessage[] = [];
  let answer: unknown = null;
  const { sdk, mcp } = standIn(async function* (call) {
    answer = await call("delete_project", {});
    yield { type: "result", subtype: "success", is_error: false, session_id: "s4" };
  });
  await new Turn((m) => sent.push(m), sdk, mcp, {}, "agentique/test").run(start);
  assert.deepEqual(answer, { content: "Not run: delete_project is not one of Agentique's tools.", isError: true });
  assert.ok(!sent.some((m) => m.type === "tool_call"));
});

test("equal inputs match whatever the order of their keys", () => {
  assert.equal(canonical({ b: 1, a: [{ d: 2, c: 3 }] }), canonical({ a: [{ c: 3, d: 2 }], b: 1 }));
  assert.notEqual(canonical({ a: 1 }), canonical({ a: "1" }));
});
