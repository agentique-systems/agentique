// A scripted stand-in for the Claude Agent companion (protocol 2) playing a
// cycle's roles for the Orchestrator's tests: no SDK, no network, no key.
// The role comes from the system prompt ("Your role: lead."). The
// implementer's first attempt leaks a configured key into the change, so the
// key gate fails and a repair round follows; the repair removes it.
import { createInterface } from "node:readline";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const KEY = "sk-fake-0123456789abcdef";
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

out({ type: "ready", protocol: 2, sdk: "test", claudeCode: "test", node: process.version });
const start = await next();
if (start.type !== "start") process.exit(5);
const o = start.options;
const role = (o.systemPrompt.match(/Your role: (?:independent )?(\w+)/) ?? [])[1] ?? "unknown";
const session = `session-${role}-${Date.now()}`;
out({
  type: "init",
  sessionId: session,
  model: "deepseek-v4-pro",
  claudeCode: "test",
  tools: ["Read", "Edit", "Bash", "mcp__agentique__read_model"],
  mcpServers: [{ name: "agentique", status: "connected" }],
  permissionMode: "default",
  apiKeySource: "ANTHROPIC_API_KEY",
  skills: [],
  agents: [],
  plugins: [],
});

let n = 0;
const call = async (name, input) => {
  n += 1;
  const id = `toolu_${n}`;
  out({ type: "assistant", id: `msg_${n}`, model: "deepseek-v4-pro", content: [
    { type: "tool_use", id, name: `mcp__agentique__${name}`, input },
  ] });
  out({ type: "tool_call", call: `c${n}`, toolUseId: id, name, input });
  for (;;) {
    const message = await next();
    if (message.type === "tool_result") return message;
    if (message.type === "eof" || message.type === "interrupt") process.exit(0);
  }
};

const modelled = existsSync(join(o.cwd, "model"));

switch (role) {
  case "lead": {
    const answer = await call("submit_proposal", {
      title: "Add an improvement note",
      kind: "correctness",
      why: "The repository has no note of its improvement (README.md:1).",
      // With a model (C-55), its names resolve there; SERVES-PART names a
      // part where a requirement is due, which is refused.
      serves: modelled ? [existsSync(join(o.cwd, "SERVES-PART")) ? "Shop::Store" : "Shop::Fast"] : ["Demo::NotesKept"],
      benefit: "The Operator reads what improved.",
      complexity: "Adds one file; nothing at the root changes.",
      parts: modelled ? ["Shop::Store"] : ["Demo"],
      plan: ["Write IMPROVEMENT.md", "Check it is there"],
      criteria: [
        { id: "c1", statement: "The note says the repository improved", check: { kind: "command", program: ["node", "--test", "note.test.mjs"] } },
      ],
    });
    out({ type: "assistant", id: "msg_end", model: "deepseek-v4-pro", content: [{ type: "text", text: `Proposed (${answer.isError ? "refused" : "accepted"}).` }] });
    break;
  }
  case "implementer": {
    // HOLD-ONCE in the repository: the first implementer works until it is
    // interrupted (the Operator closes Agentique), and the next goes on.
    if (existsSync(join(o.cwd, "HOLD-ONCE")) && !existsSync(join(o.cwd, ".held"))) {
      writeFileSync(join(o.cwd, ".held"), "");
      out({ type: "text", text: "Working on it." });
      for (;;) {
        const message = await next();
        if (message.type === "interrupt" || message.type === "eof") break;
      }
      out({ type: "error", kind: "interrupted", message: "stopped" });
      setTimeout(() => process.exit(0), 50);
      break;
    }
    const repairing = o.prompt.includes("Repair round");
    writeFileSync(join(o.cwd, "IMPROVEMENT.md"), repairing ? "Improved.\n" : `Improved with ${KEY}.\n`);
    if (modelled) {
      // The code of a part the proposal does not name (C-55).
      writeFileSync(join(o.cwd, "src", "cart.rs"), "fn cart() { checkout(); }\n");
    }
    // EDIT-ROADMAP and EDIT-PURPOSE: a change no cycle may make (C-55).
    if (existsSync(join(o.cwd, "EDIT-ROADMAP"))) {
      writeFileSync(join(o.cwd, "ROADMAP.md"), "A new direction.\n");
    }
    if (existsSync(join(o.cwd, "EDIT-PURPOSE"))) {
      const shop = join(o.cwd, "model", "Shop.sysml");
      writeFileSync(shop, readFileSync(shop, "utf8").replace("Shops sell.", "Shops sell, and lend."));
    }
    writeFileSync(
      join(o.cwd, "note.test.mjs"),
      // On the base it fails by an assertion (there is no note), which is
      // evidence; a missing file's error would not be (C-54).
      'import { test } from "node:test";\nimport assert from "node:assert";\nimport { existsSync, readFileSync } from "node:fs";\ntest("the note says it improved", () => {\n  assert.ok(existsSync("IMPROVEMENT.md"), "there is a note");\n  assert.equal(readFileSync("IMPROVEMENT.md", "utf8").trim(), "Improved.");\n});\n',
    );
    await call("submit_implementation", { summary: repairing ? "Removed the key from the note." : "Wrote the note." });
    out({ type: "assistant", id: "msg_end", model: "deepseek-v4-pro", content: [{ type: "text", text: "Implemented." }] });
    break;
  }
  case "reviewer": {
    const leaked = o.prompt.includes(KEY);
    await call("submit_review", {
      verdict: leaked ? "request_changes" : "approve",
      findings: leaked ? ["IMPROVEMENT.md:1 holds a key"] : [],
      test_changes_accepted: false,
      traceability: "none listed",
      purpose: "It still serves the purpose: a note, nothing at the root.",
    });
    out({ type: "assistant", id: "msg_end", model: "deepseek-v4-pro", content: [{ type: "text", text: "Reviewed." }] });
    break;
  }
  default:
    out({ type: "assistant", id: "msg_end", model: "deepseek-v4-pro", content: [{ type: "text", text: `No script for ${role}.` }] });
}
// The usage of the model the session was started on, and of a subagent on
// the fast model (C-54: spend by role and by model within it).
const used = o.model ?? "deepseek-v4-pro";
const usageByModel = { [used]: { inputTokens: 1000, outputTokens: 100, cacheReadTokens: 0, cacheWriteTokens: 0 } };
if (used !== "deepseek-flash") {
  usageByModel["deepseek-flash"] = { inputTokens: 10, outputTokens: 5, cacheReadTokens: 0, cacheWriteTokens: 0 };
}
out({
  type: "result",
  isError: false,
  subtype: "success",
  stopReason: "end_turn",
  numTurns: 2,
  costUsd: 0.01,
  usage: { inputTokens: 1000, outputTokens: 100, cacheReadTokens: 0, cacheWriteTokens: 0 },
  usageByModel,
  sessionId: session,
  denials: [],
  errors: [],
});
setTimeout(() => process.exit(0), 50);
