// A scripted stand-in for the Claude Agent companion (protocol 2), for the
// Assistant's tests of the runtime boundary: no SDK, no network, no key. The
// scenario is the first word of the turn's prompt after "scenario:".
import { createInterface } from "node:readline";

const out = (message) => process.stdout.write(JSON.stringify(message) + "\n");
const lines = createInterface({ input: process.stdin });
const queue = [];
let wake = null;
lines.on("line", (line) => {
  queue.push(JSON.parse(line));
  if (wake) {
    wake();
    wake = null;
  }
});
lines.on("close", () => {
  queue.push({ type: "eof" });
  if (wake) wake();
});
const next = async () => {
  while (queue.length === 0) {
    await new Promise((resolve) => (wake = resolve));
  }
  return queue.shift();
};

const init = (extra = {}) =>
  out({
    type: "init",
    sessionId: "session-1",
    model: "claude-opus-5-5",
    claudeCode: "test",
    tools: ["mcp__agentique__read_model", "mcp__agentique__apply_changes"],
    mcpServers: [{ name: "agentique", status: "connected" }],
    permissionMode: "dontAsk",
    apiKeySource: "ANTHROPIC_API_KEY",
    skills: [],
    agents: [],
    plugins: [],
    ...extra,
  });
const result = (extra = {}) =>
  out({
    type: "result",
    isError: false,
    subtype: "success",
    stopReason: "end_turn",
    numTurns: 2,
    costUsd: 0.01,
    usage: { inputTokens: 100, outputTokens: 20, cacheReadTokens: 0, cacheWriteTokens: 0 },
    sessionId: "session-1",
    denials: [],
    errors: [],
    ...extra,
  });

out({ type: "ready", protocol: 2, sdk: "test", claudeCode: "test", node: process.version });
const start = await next();
if (start.type !== "start") process.exit(5);
const prompt = start.options.prompt;
const name = (prompt.match(/scenario:(\w+)/) ?? [])[1] ?? "tool";

switch (name) {
  case "tool": {
    init();
    out({ type: "text", text: "Let me look." });
    out({ type: "assistant", model: "claude-opus-5-5", content: [
      { type: "text", text: "Let me look." },
      { type: "tool_use", id: "toolu_1", name: "mcp__agentique__read_model", input: {} },
    ] });
    out({ type: "tool_call", call: "c1", toolUseId: "toolu_1", name: "read_model", input: {} });
    const answer = await next();
    out({ type: "assistant", model: "claude-opus-5-5", content: [
      { type: "text", text: `The Studio said: ${answer.content}` },
    ] });
    result();
    break;
  }
  case "unknown": {
    // A call the Studio never defined, with an input that does not fit.
    init();
    out({ type: "assistant", model: "claude-opus-5-5", content: [
      { type: "tool_use", id: "toolu_2", name: "mcp__agentique__apply_changes", input: { nonsense: 1 } },
    ] });
    out({ type: "tool_call", call: "c1", toolUseId: "toolu_2", name: "apply_changes", input: { nonsense: 1 } });
    const answer = await next();
    out({ type: "assistant", model: "claude-opus-5-5", content: [{ type: "text", text: `Answer: ${answer.isError} ${answer.content}` }] });
    result();
    break;
  }
  case "auth":
    init();
    out({ type: "retry", attempt: 1, error: "authentication_failed", status: 401 });
    out({ type: "error", kind: "auth", message: "Anthropic refused the API key (authentication failed)." });
    break;
  case "crash":
    init();
    out({ type: "text", text: "Half a thought" });
    process.exit(3);
  case "interrupt": {
    init();
    out({ type: "assistant", model: "claude-opus-5-5", content: [
      { type: "tool_use", id: "toolu_3", name: "mcp__agentique__read_model", input: {} },
    ] });
    out({ type: "tool_call", call: "c1", toolUseId: "toolu_3", name: "read_model", input: {} });
    // Wait for the interrupt, ignoring a late answer.
    for (;;) {
      const message = await next();
      if (message.type === "interrupt" || message.type === "eof") break;
    }
    out({ type: "error", kind: "interrupted", message: "stopped by the Operator" });
    break;
  }
  case "policy":
    // The SDK started with more than Agentique allows.
    init({ tools: ["Bash", "mcp__agentique__read_model"], permissionMode: "default" });
    for (;;) {
      const message = await next();
      if (message.type === "interrupt" || message.type === "eof") break;
    }
    out({ type: "error", kind: "interrupted", message: "stopped" });
    break;
  case "resume":
    init({ sessionId: start.options.resume ?? "session-new" });
    out({ type: "assistant", model: "claude-opus-5-5", content: [
      { type: "text", text: `resume=${start.options.resume}; continues=${prompt.includes("continues your earlier session")}; handover=${prompt.includes("visible history")}` },
    ] });
    result({ sessionId: start.options.resume ?? "session-new" });
    break;
  case "environment":
    init();
    out({ type: "assistant", model: "claude-opus-5-5", content: [
      { type: "text", text: `key=${process.env.ANTHROPIC_API_KEY}; token=${process.env.GITHUB_TOKEN ?? "none"}; parent=${process.env.CLAUDECODE ?? "none"}` },
    ] });
    result();
    break;
  // Protocol 2's development sessions (C-53).
  case "development": {
    const o = start.options;
    init({ tools: ["Bash", "Read", "Edit", "mcp__agentique__read_model"], permissionMode: "default" });
    out({ type: "assistant", model: "deepseek-v4-pro", content: [
      { type: "text", text: `undecided=${o.policy?.undecided}; protectsModel=${o.policy?.protected.includes("model/*.sysml")}; preset=${o.preset}; endpoint=${o.endpoint?.baseUrl}; fast=${o.endpoint?.fastModel}; sources=${o.settingSources.join(",")}; cwdIsProject=${o.cwd.endsWith("project")}; agents=${Object.keys(o.agents).join(",")}; key=${process.env.ANTHROPIC_API_KEY}; path=${(process.env.PATH ?? process.env.Path ?? "").length > 0}; manifest=${process.env.CARGO_MANIFEST_DIR ? "kept" : "none"}; parent=${process.env.CLAUDECODE ?? "none"}` },
    ] });
    result();
    break;
  }
  case "builtin":
    // One of the SDK's own tools: its result comes back as tool_done.
    init({ tools: ["Bash", "mcp__agentique__read_model"], permissionMode: "default" });
    out({ type: "assistant", model: "deepseek-v4-pro", content: [
      { type: "tool_use", id: "toolu_b1", name: "Bash", input: { command: "cargo test" } },
    ] });
    out({ type: "task", event: "started", id: "k1", description: "Review the change", agent: "reviewer", status: null, summary: null });
    out({ type: "task", event: "done", id: "k1", description: "Review the change", agent: "reviewer", status: "completed", summary: "Looks right" });
    out({ type: "tool_done", toolUseId: "toolu_b1", isError: true, content: "test result: FAILED. 1 failed" });
    out({ type: "compaction", trigger: "auto", preTokens: 180000, postTokens: 12000 });
    out({ type: "assistant", model: "deepseek-v4-pro", content: [{ type: "text", text: "One test fails." }] });
    result();
    break;
  case "lingering": {
    // The turn answers, but background work keeps it open: the Studio's
    // pause gate still reaches it.
    init({ tools: ["mcp__agentique__read_model"], permissionMode: "default" });
    result();
    let mode = "none";
    const deadline = Date.now() + 4000;
    while (Date.now() < deadline) {
      const message = await Promise.race([next(), new Promise((r) => setTimeout(() => r(null), 200))]);
      if (message && message.type === "gate") {
        mode = message.mode;
        break;
      }
    }
    out({ type: "assistant", model: "deepseek-v4-pro", content: [{ type: "text", text: `gate=${mode}` }] });
    out({ type: "done" });
    break;
  }
  case "parallel":
    // One reply, sent as one message per content block (as the SDK does),
    // with two calls of the SDK's own tools that run side by side.
    init({ tools: ["Read", "mcp__agentique__read_model"], permissionMode: "default" });
    out({ type: "assistant", id: "msg_1", model: "deepseek-v4-pro", content: [
      { type: "tool_use", id: "toolu_p1", name: "Read", input: { file_path: "a.rs" } },
    ] });
    out({ type: "assistant", id: "msg_1", model: "deepseek-v4-pro", content: [
      { type: "tool_use", id: "toolu_p2", name: "Read", input: { file_path: "b.rs" } },
    ] });
    out({ type: "tool_done", toolUseId: "toolu_p1", isError: false, content: "fn a() {}" });
    out({ type: "tool_done", toolUseId: "toolu_p2", isError: false, content: "fn b() {}" });
    out({ type: "assistant", id: "msg_2", model: "deepseek-v4-pro", content: [{ type: "text", text: "Both read." }] });
    result();
    break;
  case "permission": {
    init({ tools: ["Read", "mcp__agentique__read_model"], permissionMode: "default" });
    out({ type: "permission", call: "p1", tool: "Read", input: { file_path: "D:\\notes.txt" }, reason: "D:/notes.txt is outside the folders this session may read" });
    let answer;
    for (;;) {
      answer = await next();
      if (answer.type === "permission_result" || answer.type === "eof") break;
    }
    out({ type: "assistant", model: "deepseek-v4-pro", content: [{ type: "text", text: `allow=${answer.allow}; message=${answer.message}` }] });
    result();
    break;
  }
  case "steer": {
    // The Studio queues a message and changes the gate while the turn runs.
    init({ tools: ["mcp__agentique__read_model"], permissionMode: "default" });
    const seen = [];
    while (seen.length < 2) {
      const message = await next();
      if (message.type === "eof") break;
      if (message.type === "message") seen.push(`message:${message.text}`);
      if (message.type === "gate") seen.push(`gate:${message.mode}`);
    }
    out({ type: "paused", tool: "Bash" });
    out({ type: "assistant", model: "deepseek-v4-pro", content: [{ type: "text", text: `seen=${seen.sort().join("|")}` }] });
    result();
    break;
  }
}
setTimeout(() => process.exit(0), 50);
